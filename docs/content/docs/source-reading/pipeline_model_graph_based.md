+++
title = "Databend 源码阅读： 图解pipeline调度模型"
description = "“Databend 源码阅读”系列文章的第四篇，本文我们不深入看太多源代码,而是从pipeline调度上出发,为大家深入解读Databend基于work-steal和状态机的并发调度模型"
draft = false
weight = 430
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
giscus = true

+++

作者：[JackTan25](https://github.com/JackTan25) | Databend Contributor

## 一.基于图的初始化
![](imgs/1.pipeline%E5%9F%BA%E6%9C%AC%E7%BB%93%E6%9E%84.png)
```text
    上图便是databend的一条pipeline结构,通常对于每一个PipeItem,这里只会有一个input_port和output_port,一个Pipe当中的PipeItem的数量则通常代表着并行度.每一个PipeItem里面对应着一个算子(不过在有些情况下并不一定一个pipeItem就只有一对input_port和output_port,所以上图画的更加广泛一些),算子的推进由调度模型来触发
```
将pipeline初始化为graph:这里细致展示下生成的过程
```text
    databend采取的是采取的是StableGraph这个结构,我们最开始是得到了下面第一张图这样的Pipeline,我们最后生成的是下面第二张图的graph.
```
![](imgs/2.pipeline-graph%E6%9E%84%E5%BB%BA(1).jpg)
![](imgs/3.pipeline-graph%E6%9E%84%E5%BB%BA(2).jpg)
```text
    上面第二张图的的连接只是一个物理上的单纯图的连接,但是node内部pipe_item对应的port没有对接起来.我们还需要关心的是具体如何把对应的port给连接起来的.在构建图的时候每一个PipeItem包装为一个node,包装的过程是以Pipe为顺序的.这样我们就为上面每一个PipeItem都加上了一个Node编号,后面我们需要按照为对应的input_port和output_port去加上edge,我们的连接是一个平行的连接.
```
我们将构建过程当中需要使用到的结构做一个介绍:
```rust
// 一个Node对应一个PipeItem
struct Node {
    // node的状态记录,其实应该理解为是记录
    // pipeline执行过程当中一个最小执行
    // 单元PipeItem的状态,一共有如下三种状态:
    // Idle,Processing,Finished,
    state: std::sync::Mutex<State>,
    
    updated_list: Arc<UpdateList>,
    // 一下是pipeItem的内容
    inputs_port: Vec<Arc<InputPort>>,
    outputs_port: Vec<Arc<OutputPort>>,
    processor: ProcessorPtr,
}

pub struct UpdateList {
    inner: UnsafeCell<UpdateListMutable>,
}
pub struct UpdateListMutable {
    // update_input与update_output调用时更新,用于
    // 调度模型的任务调度
    updated_edges: Vec<DirectedEdge>,
    // 对于Node而言,其上的每一个input_port和output_port都会对应
    // 一个trigger,我们从edge0,edge1,...,edgen（编号就是上图示例）
    // 这样每次给source_node为其对应的output_port添加一个trigger
    // 为target_node的input_port添加一个trigger
    updated_triggers: Vec<Arc<UnsafeCell<UpdateTrigger>>>,
}
// trigger的作用就是用来后面调度模型推进pipeline向下
// 执行调度任务使用的
pub struct UpdateTrigger {
 // 记录该trigger对应的是哪一个边
 index: EdgeIndex,
 // 记录其属于哪一个UpdateListMutable
 update_list: *mut UpdateListMutable,
 // 初始化为0
 version: usize,
 // 初始化为0
 prev_version: usize,
}
// 上面的例子最后我们得到的graph初始化后应该是下面这样
```
![](imgs/4.pipeline-graph%E6%9E%84%E5%BB%BA(3).jpg)
```rust
// 而对于input_port和output_port的数据的传递,则是两者之间共享一个SharedData
pub struct SharedStatus {
    // SharedData按照8字节对齐,所以其地址
    // 最后三位永远为0,在这里我们会利用这三
    // 位来标记当前port的状态,一共有三种
    // NEED_DATA,HAS_DATA,IS_FINISHED
    data: AtomicPtr<SharedData>,
}
```
## 二.基于work-steal与状态机的并发调度模型
```text
    初始化调度是将我们的graph的所有出度为0的Node作为第一次任务调度节点,对应我们的例子就是Node4,Node5每一次调度都是抽取出graph当中的同步任务和异步任务,下图是pipeline的调度模型，用于抽取出当前graph当中可执行的同步processor和异步processor，调度模型的输入是最上面的graph,而输出则是sync_processor_queue和async_processor_queue,无论是在初始化时还是在后面继续执行的过程都是利用的下面的调度模型来进行调度.调度模型的执行终点是need_schedule_nodes和need_schedule_edges均为空
```
![](imgs/5.%E8%B0%83%E5%BA%A6%E6%A8%A1%E5%9E%8B.jpg)

## 三.执行模型
执行模型对应相关结构如下:
```rust
struct ExecutorTasks {
    // 记录当前还剩余的task的数量
    tasks_size: usize,
    workers_waiting_status: WorkersWaitingStatus,
    // 记录同步任务,其大小等于系统当前允许的thread数量
    workers_sync_tasks: Vec<VecDeque<ProcessorPtr>>,
    // 记录已完成的异步任务,其大小等于系统当前允许的thread数量
    workers_completed_async_tasks: Vec<VecDeque<CompletedAsyncTask>>,
}

// 用于记录等待线程和活跃线程
pub struct WorkersWaitingStatus {
    stack: Vec<usize>,
    stack_size: usize,
    worker_pos_in_stack: Vec<usize>,
}

pub struct WorkersCondvar {
    // 记录还未执行完的异步任务
    waiting_async_task: AtomicUsize,
    // 用于唤醒机制
    workers_condvar: Vec<WorkerCondvar>,
}

pub struct ProcessorAsyncTask {
    // worker_id代表的是当前异步任务对应的线程id
    worker_id: usize,
    // 在graph当中的节点位置
    processor_id: NodeIndex,
    // 全局的任务队列
    queue: Arc<ExecutorTasksQueue>,
    // 用于work-steal的调度唤醒策略
    workers_condvar: Arc<WorkersCondvar>,
    // 一个包装future,见下图的具体包装
    inner: BoxFuture<'static, Result<()>>,
}

pub struct ExecutorTasksQueue {
    // 记录当前执行任务队列是否完成
    finished: Arc<AtomicBool>,
    // 通知异步任务结束
    finished_notify: Arc<Notify>,
    workers_tasks: Mutex<ExecutorTasks>,
}

// 唤醒机制
struct WorkerCondvar {
    mutex: Mutex<bool>,
    condvar: Condvar,
}
```
执行模型的流程图如下:
![](imgs/6.%E5%B9%B6%E5%8F%91%E6%89%A7%E8%A1%8C%E6%A8%A1%E5%9E%8B.jpg)
## 四. 限时机制
```text
    限时机制其实是比较简单的,其主要的作用就是限制sql的pipeline执行的时间在规定时间内完成,
如果超时则自动终止.这个机制底层实现就是用了一个异步任务来跟踪,一旦超时就通知整个执行模型结束,这里对应的就是执行模型流程图里面的finish.
```
以上便是databend的机遇状态机和work-steal机制的并发调度模型实现.