+++
title = "Databend 源码阅读： pipeline 的执行"
description = "“Databend 源码阅读”系列文章的第三篇，以一条 SQL 的 pipeline 为例，帮助大家了解 databend 中 pipeline 的执行过程"
draft = false
weight = 430
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
giscus = true

+++

作者：[Dousir9](https://github.com/dousir9) | Databend Contributor

## 一条 SQL 的 pipeline

本篇文章将以一条 SQL `select t.id from t group by t.id` 为例，分析 Pipeline 的执行，表结构及该 SQL 的 pipeline 如下所示，我们将从底部的 `SyncReadParquetDataSource` 向上进行分析。

```sql
mysql> desc t;
+-------+------+------+---------+-------+
| Field | Type | Null | Default | Extra |
+-------+------+------+---------+-------+
| id    | INT  | NO   | 0       |       |
| val   | INT  | NO   | 0       |       |
+-------+------+------+---------+-------+

mysql> explain pipeline select t.id from t group by t.id;
+--------------------------------------------------------+
| explain                                                |
+--------------------------------------------------------+
| CompoundBlockOperator(Project) × 1 processor           |
|   TransformFinalGroupBy × 1 processor                  |
|     TransformSpillReader × 1 processor                 |
|       TransformPartitionBucket × 1 processor           |
|         TransformGroupBySpillWriter × 1 processor      |
|           TransformPartialGroupBy × 1 processor        |
|             DeserializeDataTransform × 1 processor     |
|               SyncReadParquetDataSource × 1 processor  |
+--------------------------------------------------------+
```

## execute_single_thread

首先我们需要明白 `PipelineExecutor` 是怎么运作的

```rust
// src/query/service/src/pipelines/executor/pipeline_executor.rs
impl PipelineExecutor {
    // ...

    /// # Safety
    ///
    /// Method is thread unsafe and require thread safe call
    pub unsafe fn execute_single_thread(&self, thread_num: usize) -> Result<()> {
        let workers_condvar = self.workers_condvar.clone();
        let mut context = ExecutorWorkerContext::create(
            thread_num,
            workers_condvar,
            self.settings.query_id.clone(),
        );

        while !self.global_tasks_queue.is_finished() {
            // When there are not enough tasks, the thread will be blocked, so we need loop check.
            while !self.global_tasks_queue.is_finished() && !context.has_task() {
                self.global_tasks_queue.steal_task_to_context(&mut context);
            }

            while !self.global_tasks_queue.is_finished() && context.has_task() {
                if let Some(executed_pid) = context.execute_task()? {
                    // Not scheduled graph if pipeline is finished.
                    if !self.global_tasks_queue.is_finished() {
                        // We immediately schedule the processor again.
                        let schedule_queue = self.graph.schedule_queue(executed_pid)?;
                        schedule_queue.schedule(&self.global_tasks_queue, &mut context, self);
                    }
                }
            }
        }

        Ok(())
    }
    // ...
}
```

### 初始化线程

在调用 `from_pipelines` 构建 `PipelineExecutor` 时，我们会遍历每个 `Pipeline` 的 `get_max_threads` 来获得当前这个 `PipelineExecutor` 所需的线程数 `threads_num`。然后在 `execute_threads` 函数中，我们会创建 `threads_num` 个线程，每个线程都有当前这个 `PipelineExecutor` 的一份拷贝，随后每个线程会调用 `execute_single_thread` 开始执行任务。 

### 执行

**（1）首先获得一份条件变量 `workers_condvar` 的拷贝并用它来创建一个 `ExecutorWorkerContext`，它存有 query_id，worker_num：worker 编号，task：当前要执行的任务，workers_condvar。**

**（2）当 `global_tasks_queue` 没有结束时，就会一直循环，如果 `context` 中没有 task，则会调用 `steal_task_to_context` 来获取任务，如果没有获取到则阻塞等待被唤醒。**

**（3）当获取到任务时，会首先调用 `execute_task` 来执行任务，对于 `ExecutorTask::Sync` 类型的任务来说，会调用 `execute_sync_task` 进而调用 `Processor` 的 `process` 函数，然后返回 `processor.id()` 用来后续推动 pipeline 的执行；而当 task 的类型为 `ExecutorTask::AsyncCompleted` 时，表示一个异步任务执行完了，这时我们返回 `task.id` 用来后续推动 pipeline 的执行。**

```rust
// src/query/service/src/pipelines/executor/executor_worker_context.rs
impl ExecutorWorkerContext {
    pub unsafe fn execute_task(&mut self) -> Result<Option<NodeIndex>> {
        match std::mem::replace(&mut self.task, ExecutorTask::None) {
            ExecutorTask::None => Err(ErrorCode::Internal("Execute none task.")),
            ExecutorTask::Sync(processor) => self.execute_sync_task(processor),
            ExecutorTask::AsyncCompleted(task) => match task.res {
                Ok(_) => Ok(Some(task.id)),
                Err(cause) => Err(cause),
            },
        }
    }
}
```

**（4）在调用 `execute_task` 后我们得到了一个 `executed_pid`，这时候我们需要拿这个 `executor_pid` 来做一些 schedule 工作，继续推动 pipeline 的执行，首先调用 `schedule_queue`。**

```rust
// src/query/service/src/pipelines/executor/executor_graph.rs
impl ExecutingGraph {
    // ...

    /// # Safety
    ///
    /// Method is thread unsafe and require thread safe call
    pub unsafe fn schedule_queue(
        locker: &StateLockGuard,
        index: NodeIndex,
        schedule_queue: &mut ScheduleQueue,
    ) -> Result<()> {
        let mut need_schedule_nodes = VecDeque::new();
        let mut need_schedule_edges = VecDeque::new();

        need_schedule_nodes.push_back(index);
        while !need_schedule_nodes.is_empty() || !need_schedule_edges.is_empty() {
            // To avoid lock too many times, we will try to cache lock.
            let mut state_guard_cache = None;

            if need_schedule_nodes.is_empty() {
                let edge = need_schedule_edges.pop_front().unwrap();
                let target_index = DirectedEdge::get_target(&edge, &locker.graph)?;

                let node = &locker.graph[target_index];
                let node_state = node.state.lock().unwrap();

                if matches!(*node_state, State::Idle) {
                    state_guard_cache = Some(node_state);
                    need_schedule_nodes.push_back(target_index);
                }
            }

            if let Some(schedule_index) = need_schedule_nodes.pop_front() {
                let node = &locker.graph[schedule_index];

                if state_guard_cache.is_none() {
                    state_guard_cache = Some(node.state.lock().unwrap());
                }
                let event = node.processor.event()?;
                if tracing::enabled!(tracing::Level::TRACE) {
                    tracing::trace!(
                        "node id: {:?}, name: {:?}, event: {:?}",
                        node.processor.id(),
                        node.processor.name(),
                        event
                    );
                }
                let processor_state = match event {
                    Event::Finished => State::Finished,
                    Event::NeedData | Event::NeedConsume => State::Idle,
                    Event::Sync => {
                        schedule_queue.push_sync(node.processor.clone());
                        State::Processing
                    }
                    Event::Async => {
                        schedule_queue.push_async(node.processor.clone());
                        State::Processing
                    }
                };

                node.trigger(&mut need_schedule_edges);
                *state_guard_cache.unwrap() = processor_state;
            }
        }

        Ok(())
    }
}
```

在介绍 `schedule_queue` 函数之前有几个概念，`trait Processor` 有 `event`，`process`，`async_process` 这些函数，`event` 的作用是根据当前这个 Processor 的信息，来推动这个 Processor：包括改变 Processor 中的变量，改变 input port 和 output port，`event` 会返回一个 `Event` 状态来指示下一步的工作：

+ `Event::Finished`：表示 Processor 的工作结束了，将 Processor 的状态设置为 `State::Finished`
+ `Event::NeedData | Event::NeedConsume`：表示 Processor 的 input 需要数据或者 output 的数据需要被消费，将 Processor 的状态设置为 `tate::Idle`，表示需要进行 schedule。
+ `Event::Sync`：表示 Processor 需要调用 `process` 进行处理，将 Processor push 到 `schedule_queue` 的 `sync_queue` 中，并将 Processor 状态设置为 `State::Processing`。
+ `Event::Async`：表示 Processor 需要调用 `async_process` 进行处理，将 Processor push 到 `schedule_queue` 的 `async_queue` 中，并将 Processor 状态设置为 `State::Processing`。

schedule_queue 的工作过程：

1. 首先初始化两个 VecDeque： `need_schedule_nodes: VecDeque<NodeIndex>` 和 `need_schedule_edges: VecDeque<DirectedEdge>` 分别用来存放需要进行 schedule 的 NodeIndex 和 DirectedEdge，然后将 `executor_pid` push `need_schedule_nodes` 中。
2. 只要这两个 VecDeque 任意一个不为空，我们就需要不断地进行 schedule。
3. 每次 schedule 时，首先我们会判断 `need_schedule_nodes` 是否为空，如果它为空，那 `need_schedule_edges` 一定不为空，此时我们从 `need_schedule_edges` 中 pop 出一条 `DirectedEdge` edge，然后获得这条 edge 的 target node（注意这个 target node 不是 edge 的指向，`DirectedEdge` 有两种类型：`Source` 和 `Target`，当 Processor 的 input 改变时，会在 trigger 的 update_list 中 push 一条 `DirectedEdge::Target(self_.index)`，而如果是 Processor 的 output 改变，则 push 一条 `DirectedEdge::Source(self_.index)`），如果 target node 的状态为 `State::Idle`，表示它在上一次调用 `event` 时返回的 Event 状态为 `Event::NeedData` 或 `Event::NeedConsume`，即它上次 `event` 时 input 需要数据或 output 数据需要被消费，而它现在的状态可能是 input 的数据已经来了或者 output 的数据被消费了，因此我们需要将其 push 到 `need_schedule_nodes` 中来再次调用 `event` 看看是否可以推动这个 Processor。
4. 然后我们尝试从 `need_schedule_nodes` pop 出一个 NodeIndex，并从 `ExecutingGraph` 中得到这个 Node，然后调用它的 Processor 的 `event`，然后根据返回的 `Event` 状态来进行下一步工作（如开头描述）。
5. 最后调用这个 Node 的 trigger 函数，将 updated_list 中的 `DirectedEdge` 都 push 到 `need_schedule_edges` 中。
6. 如果 `need_schedule_nodes` 或 `need_schedule_edges` 不为空则开始下一次 schedule。
7. schedule 结束，将 `schedule_queue` 返回。

**（5）调用 `schedule_queue.schedule` 处理 schedule_queue 中的 tasks**

```rust
// src/query/service/src/pipelines/executor/executor_graph.rs
impl ScheduleQueue {
    // ...
    pub fn schedule(
        mut self,
        global: &Arc<ExecutorTasksQueue>,
        context: &mut ExecutorWorkerContext,
        executor: &PipelineExecutor,
    ) {
        debug_assert!(!context.has_task());

        while let Some(processor) = self.async_queue.pop_front() {
            Self::schedule_async_task(
                processor,
                context.query_id.clone(),
                executor,
                context.get_worker_num(),
                context.get_workers_condvar().clone(),
                global.clone(),
            )
        }

        if !self.sync_queue.is_empty() {
            self.schedule_sync(global, context);
        }

        if !self.sync_queue.is_empty() {
            self.schedule_tail(global, context);
        }
    }
    // ...
}
```

1. 对于 `async_queue` 中的 Processor，我们将其 push 到 async_runtime 中，当 Processor 调用 `async_process` 完成异步任务完成后，会将 `CompletedAsyncTask` push 到 `global_tasks_queue` 中。
2. 对于 `sync_queue` 中的 Processor，我们首先调用 `schedule_sync` 取出一个 Processor 并把包装为一个 `ExecutorTask::Sync(processor)` 任务交给当前线程继续执行。然后将剩下的 Processor 都包装为 `Processor` push 到 `global_tasks_queue` 中，让其他线程取出 task 并行执行。

## SyncSourcer

```rust
// src/query/pipeline/source/src/sync_source.rs
#[async_trait::async_trait]
impl<T: 'static + SyncSource> Processor for SyncSourcer<T> {
    fn name(&self) -> String {
        T::NAME.to_string()
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn event(&mut self) -> Result<Event> {
        if self.is_finish {
            self.output.finish();
            return Ok(Event::Finished);
        }

        if self.output.is_finished() {
            return Ok(Event::Finished);
        }

        if !self.output.can_push() {
            return Ok(Event::NeedConsume);
        }

        match self.generated_data.take() {
            None => Ok(Event::Sync),
            Some(data_block) => {
                self.output.push_data(Ok(data_block));
                Ok(Event::NeedConsume)
            }
        }
    }

    fn process(&mut self) -> Result<()> {
        match self.inner.generate()? {
            None => self.is_finish = true,
            Some(data_block) => {
                let progress_values = ProgressValues {
                    rows: data_block.num_rows(),
                    bytes: data_block.memory_size(),
                };
                self.scan_progress.incr(&progress_values);
                self.generated_data = Some(data_block)
            }
        };

        Ok(())
    }
}
```

```rust
// src/query/storages/fuse/src/operations/read/parquet_data_source_reader.rs
impl SyncSource for ReadParquetDataSource<true> {
    const NAME: &'static str = "SyncReadParquetDataSource";

    fn generate(&mut self) -> Result<Option<DataBlock>> {
        match self.partitions.steal_one(self.id) {
            None => Ok(None),
            Some(part) => Ok(Some(DataBlock::empty_with_meta(DataSourceMeta::create(
                vec![part.clone()],
                vec![self.block_reader.sync_read_columns_data_by_merge_io(
                    &ReadSettings::from_ctx(&self.partitions.ctx)?,
                    part,
                )?],
            )))),
        }
    }
}
```

### process

首先调用 inner （例如 `ReadParquetDataSource`，它实现了 trait SyncSource) 的 `generate` 获得一个空的 `DataBlock`，这个 `DataBlock` 数据为空，但是 `meta` 不为空，存有 `part` 和 `data`。将这个 `data_block` 赋值给 `self.generated_data`，

### event

在下一次调用 `event` 的时候将 `self.generated_data` 通过 `self.output.push_data(Ok(data_block))` 发送出去，并返回 `Event::NeedConsume` 这个状态。如果 `!self.output.can_push()` 为 true 的话，说明现在有 data_block 在 output 中，返回 `Event::NeedConsume` 状态。

## DeserializeDataTransform

```rust
// src/query/storages/fuse/src/operations/read/parquet_data_source_deserializer.rs
#[async_trait::async_trait]
impl Processor for DeserializeDataTransform {
    fn name(&self) -> String {
        String::from("DeserializeDataTransform")
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn event(&mut self) -> Result<Event> {
        if self.output.is_finished() {
            self.input.finish();
            self.uncompressed_buffer.clear();
            return Ok(Event::Finished);
        }

        if !self.output.can_push() {
            self.input.set_not_need_data();
            return Ok(Event::NeedConsume);
        }

        if let Some(data_block) = self.output_data.take() {
            self.output.push_data(Ok(data_block));
            return Ok(Event::NeedConsume);
        }

        if !self.chunks.is_empty() {
            if !self.input.has_data() {
                self.input.set_need_data();
            }

            return Ok(Event::Sync);
        }

        if self.input.has_data() {
            let mut data_block = self.input.pull_data().unwrap()?;
            if let Some(source_meta) = data_block.take_meta() {
                if let Some(source_meta) = DataSourceMeta::downcast_from(source_meta) {
                    self.parts = source_meta.part;
                    self.chunks = source_meta.data;
                    return Ok(Event::Sync);
                }
            }

            unreachable!();
        }

        if self.input.is_finished() {
            self.output.finish();
            self.uncompressed_buffer.clear();
            return Ok(Event::Finished);
        }

        self.input.set_need_data();
        Ok(Event::NeedData)
    }

    fn process(&mut self) -> Result<()> {
        let part = self.parts.pop();
        let chunks = self.chunks.pop();
        if let Some((part, read_res)) = part.zip(chunks) {
            let start = Instant::now();

            let columns_chunks = read_res.columns_chunks()?;
            let part = FusePartInfo::from_part(&part)?;

            let data_block = self.block_reader.deserialize_parquet_chunks_with_buffer(
                &part.location,
                part.nums_rows,
                &part.compression,
                &part.columns_meta,
                columns_chunks,
                Some(self.uncompressed_buffer.clone()),
            )?;

            // Perf.
            {
                metrics_inc_remote_io_deserialize_milliseconds(start.elapsed().as_millis() as u64);
            }

            let progress_values = ProgressValues {
                rows: data_block.num_rows(),
                bytes: data_block.memory_size(),
            };
            self.scan_progress.incr(&progress_values);

            self.output_data = Some(data_block);
        }

        Ok(())
    }
}
```

### event

（1）如果 `self.output.is_finished()` 为 true，则调用 `self.input.finish()` 并返回 `Event::Finished`。

（2）如果 `!self.output.can_push()` 的话，表示上一次 push 出去的数据还没被消费，对 input 调用 `set_not_need_data` 表示不需要数据，返回 `Event::NeedConsume`。

（3）process 处理好的数据会放到 `self.output_data` 中，因此如果 
`self.output_data.take()` 有数据的话，则调用 `self.output.push_data(Ok(data_block))` 将它发送出去，并返回 `Event::NeedConsume`。

（4）如果 `self.input.has_data()` 为 true，即 input 有数据，则调用 `self.input.pull_data().unwrap()?` 将 data_block pull 过来，然后获取其中的 `BlockMetaInfo` 并将其 downcast 成 `DataSourceMeta`，然后给 `self.parts` 和 `self.chunks` 赋值，返回 `Event::Sync` 状态。

（5）在（4）之前如果 `!self.chunks.is_empty()` 为 true，这时候我们正在处理之前的 data_block，因此要返回 `Event::Sync` 这个状态。此外因为这时候我们已经把上一个 data_block pull 过来了，input 可能为空，如果 input 没有数据的话，我们需要将 input `set_need_data`，为下一次 pull 做准备。

（6）如果 `self.input.is_finished()` 为 ture，则调用 `self.output.finish()` 并返回 `Event::Finished`。

（7）当前 Processor 既没有结束，也没有数据，因此对 input `self.input.set_need_data()`，返回 `Event::NeedData`。

### process

每次调用 process 会处理一块 parquet_chunks，将其反序列化为数据不为空的 DataBlock，然后将其转交给 `self.output_data` 等待下一次 `event` 发送出去。

## AccumulatingTransformer

```rust
// src/query/pipeline/transforms/src/processors/transforms/transform_accmulating.rs
#[async_trait::async_trait]
impl<T: AccumulatingTransform + 'static> Processor for AccumulatingTransformer<T> {
    fn name(&self) -> String {
        String::from(T::NAME)
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn event(&mut self) -> Result<Event> {
        if self.output.is_finished() {
            if !self.called_on_finish {
                return Ok(Event::Sync);
            }

            self.input.finish();
            return Ok(Event::Finished);
        }

        if !self.output.can_push() {
            self.input.set_not_need_data();
            return Ok(Event::NeedConsume);
        }

        if let Some(data_block) = self.output_data.pop_front() {
            self.output.push_data(Ok(data_block));
            return Ok(Event::NeedConsume);
        }

        if self.input_data.is_some() {
            return Ok(Event::Sync);
        }

        if self.input.has_data() {
            self.input_data = Some(self.input.pull_data().unwrap()?);
            return Ok(Event::Sync);
        }

        if self.input.is_finished() {
            return match !self.called_on_finish {
                true => Ok(Event::Sync),
                false => {
                    self.output.finish();
                    Ok(Event::Finished)
                }
            };
        }

        self.input.set_need_data();
        Ok(Event::NeedData)
    }

    fn process(&mut self) -> Result<()> {
        if let Some(data_block) = self.input_data.take() {
            self.output_data.extend(self.inner.transform(data_block)?);
            return Ok(());
        }

        if !self.called_on_finish {
            self.called_on_finish = true;
            self.output_data.extend(self.inner.on_finish(true)?);
        }

        Ok(())
    }
}
```

### event

整体上与 `DeserializeDataTransform` 的 `event` 类似，不同的地方在于：

（1）self.output_data 的类型为 `VecDeque<DataBlock>`，而不是 `DataBlock`，可以发送数据时，从调用 `self.output_data.pop_front()` 从队头取出一个 `DataBlock` 并 push 出去。

（2）在 `self.output.is_finished()` 或 `self.input.is_finished()` 为 true 时，首先判断 `called_on_finish` 是否为 true，如果不为 true 的话，表示还没有调用 inner 的 `on_finish`，这时候返回 `Event::Sync` 而不是 `Event::Finished`。

### process

（1）如果 `input_data` 中有数据，则获取 `input_data` 中的 DataBlock 并用它调用 `inner` （例如 `TransformPartialGroupBy`，它实现了 trait `AccumulatingTransform`）的 `transform(data_block)?` 来获取需要 spill 的 data_blocks，这些 data_block 的 `columns` 是空的，但是 meta 不为空，meta 的类型为 `AggregateMeta::Spilling`；如果当前的 hash table 不大，则返回的结果是 `vec![]`，`transform` 的分析在下面。

（2）如果 `input_data` 中没有数据且 `called_on_finish` 为 false，则调用 inner 的 `on_finish` 来获取 DataBlock，同样，这些 DataBlock 的 `columns` 是空的，但是 meta 不为空，meta 的类型为 `AggregateMeta::HashTable`，`on_finish` 的分析在下面。

### transform

```rust
// src/query/service/src/pipelines/processors/transforms/aggregator/transform_group_by_partial.rs
impl<Method: HashMethodBounds> AccumulatingTransform for TransformPartialGroupBy<Method> {
    const NAME: &'static str = "TransformPartialGroupBy";

    fn transform(&mut self, block: DataBlock) -> Result<Vec<DataBlock>> {
        let block = block.convert_to_full();
        let group_columns = self
            .group_columns
            .iter()
            .map(|&index| block.get_by_offset(index))
            .collect::<Vec<_>>();

        let group_columns = group_columns
            .iter()
            .map(|c| (c.value.as_column().unwrap().clone(), c.data_type.clone()))
            .collect::<Vec<_>>();

        unsafe {
            let rows_num = block.num_rows();
            let state = self.method.build_keys_state(&group_columns, rows_num)?;

            match &mut self.hash_table {
                HashTable::MovedOut => unreachable!(),
                HashTable::HashTable(cell) => {
                    for key in self.method.build_keys_iter(&state)? {
                        let _ = cell.hashtable.insert_and_entry(key);
                    }
                }
                HashTable::PartitionedHashTable(cell) => {
                    for key in self.method.build_keys_iter(&state)? {
                        let _ = cell.hashtable.insert_and_entry(key);
                    }
                }
            };

            #[allow(clippy::collapsible_if)]
            if Method::SUPPORT_PARTITIONED {
                if matches!(&self.hash_table, HashTable::HashTable(cell)
                    if cell.len() >= self.settings.convert_threshold ||
                        cell.allocated_bytes() >= self.settings.spilling_bytes_threshold_per_proc
                ) {
                    if let HashTable::HashTable(cell) = std::mem::take(&mut self.hash_table) {
                        self.hash_table = HashTable::PartitionedHashTable(
                            PartitionedHashMethod::convert_hashtable(&self.method, cell)?,
                        );
                    }
                }

                if matches!(&self.hash_table, HashTable::PartitionedHashTable(cell) if cell.allocated_bytes() > self.settings.spilling_bytes_threshold_per_proc)
                {
                    if let HashTable::PartitionedHashTable(v) = std::mem::take(&mut self.hash_table)
                    {
                        let _dropper = v._dropper.clone();
                        let cells = PartitionedHashTableDropper::split_cell(v);
                        let mut blocks = Vec::with_capacity(cells.len());
                        for (bucket, cell) in cells.into_iter().enumerate() {
                            if cell.hashtable.len() != 0 {
                                blocks.push(DataBlock::empty_with_meta(
                                    AggregateMeta::<Method, ()>::create_spilling(
                                        bucket as isize,
                                        cell,
                                    ),
                                ));
                            }
                        }

                        let method = PartitionedHashMethod::<Method>::create(self.method.clone());
                        let new_hashtable = method.create_hash_table()?;
                        self.hash_table = HashTable::PartitionedHashTable(HashTableCell::create(
                            new_hashtable,
                            _dropper.unwrap(),
                        ));
                        return Ok(blocks);
                    }

                    unreachable!()
                }
            }
        }

        Ok(vec![])
    }
}
```

（1）首先调用 `block.convert_to_full()` 将 DataBlock 填充满：对于每个 `BlockEntry`，如果是 `Value::Scalar` 类型，则将其重复 `self.num_rows` 次转变为 `Value::Column`，如果原本就是 `Value::Column` 类型的话就简单 clone 一下。

（2）从 datablock 中获取用于 group by 的列 `group_columns: Vec<&BlockEntry>`，然后再转变为 `Vec<(Column, DataType)>`。

（3）调用 `self.method.build_keys_state(&group_columns, rows_num)` 将 `group_columns` group_columns 变为 `KeyState`：变为 unsigned 类型，

（4）调用 `build_keys_iter` 来获取 group by key 的 iter，并将每个 key 插入到 hash table 中。

（5）如果 hash table 的长度大于 `convert_threshold` 或者分配的字节数大于 `spilling_bytes_threshold_per_proc`，则将其装换为 `PartitionedHashTable`。

（6）如果一个 `PartitionedHashTable` 的长度大于 `convert_threshold` 或者分配的字节数大于 `spilling_bytes_threshold_per_proc`，这时候需要 spill 到存储上：将当前 hash table 转变为 `blocks: Vec<DataBlock>`，这些 DataBlock 的 `columns` 为空，meta 不为空，类型为：`AggregateMeta::Spilling`，然后创建一个新的 hash table，并将 blocks 返回。

（7）如果当前 hash table 不是很大，则返回 `vec![]`。

### build_keys_state

`src/query/expression/src/kernels/group_by_hash.rs`

（1）如果 group_by 只有一个字段的且这个字段是整数类型的话，则将这一列 cast 为 unsigned 类型，包装在 `KeysState` 中返回。

（2）否则调用 `build_keys_vec` 来构建 key，并将 key cast 成整数类型包装在 `KeysState` 中返回。

### on_finish

```rust
// src/query/service/src/pipelines/processors/transforms/aggregator/transform_group_by_partial.rs
impl<Method: HashMethodBounds> AccumulatingTransform for TransformPartialGroupBy<Method> {
  	// ...
    fn on_finish(&mut self, _output: bool) -> Result<Vec<DataBlock>> {
        Ok(match std::mem::take(&mut self.hash_table) {
            HashTable::MovedOut => unreachable!(),
            HashTable::HashTable(cell) => match cell.hashtable.len() == 0 {
                true => vec![],
                false => vec![DataBlock::empty_with_meta(
                    AggregateMeta::<Method, ()>::create_hashtable(-1, cell),
                )],
            },
            HashTable::PartitionedHashTable(v) => {
                let cells = PartitionedHashTableDropper::split_cell(v);
                let mut blocks = Vec::with_capacity(cells.len());
                for (bucket, cell) in cells.into_iter().enumerate() {
                    if cell.hashtable.len() != 0 {
                        blocks.push(DataBlock::empty_with_meta(
                            AggregateMeta::<Method, ()>::create_hashtable(bucket as isize, cell),
                        ));
                    }
                }

                blocks
            }
        })
    }
}
```

将 `HashTable` 或者 `PartitionedHashTable` 转变为 DataBlock 返回，这些 DataBlock 的 `columns` 字段为空，meta 字段类型为 `AggregateMeta::HashTable`。

如果 hash table 是 `HashTable::HashTable` 类型，则返回的 bucket id 为 -1，如果是 `HashTable::PartitionedHashTable`，则先调用 `split_cell` 将其 split 成 cells，然后再返回，bucket id 为 0 ~ cells.len() - 1。

## TransformGroupBySpillWriter

```rust
// src/query/service/src/pipelines/processors/transforms/aggregator/serde/transform_group_by_spill_writer.rs
#[async_trait::async_trait]
impl<Method: HashMethodBounds> Processor for TransformGroupBySpillWriter<Method> {
    fn name(&self) -> String {
        String::from("TransformGroupBySpillWriter")
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn event(&mut self) -> Result<Event> {
        if self.output.is_finished() {
            self.input.finish();
            return Ok(Event::Finished);
        }

        if !self.output.can_push() {
            self.input.set_not_need_data();
            return Ok(Event::NeedConsume);
        }

        if let Some(spilled_meta) = self.spilled_meta.take() {
            self.output
                .push_data(Ok(DataBlock::empty_with_meta(spilled_meta)));
            return Ok(Event::NeedConsume);
        }

        if self.writing_data_block.is_some() {
            self.input.set_not_need_data();
            return Ok(Event::Async);
        }

        if self.spilling_meta.is_some() {
            self.input.set_not_need_data();
            return Ok(Event::Sync);
        }

        if self.input.has_data() {
            let mut data_block = self.input.pull_data().unwrap()?;

            if let Some(block_meta) = data_block
                .get_meta()
                .and_then(AggregateMeta::<Method, ()>::downcast_ref_from)
            {
                if matches!(block_meta, AggregateMeta::Spilling(_)) {
                    self.input.set_not_need_data();
                    let block_meta = data_block.take_meta().unwrap();
                    self.spilling_meta = AggregateMeta::<Method, ()>::downcast_from(block_meta);
                    return Ok(Event::Sync);
                }
            }

            self.output.push_data(Ok(data_block));
            return Ok(Event::NeedConsume);
        }

        if self.input.is_finished() {
            self.output.finish();
            return Ok(Event::Finished);
        }

        self.input.set_need_data();
        Ok(Event::NeedData)
    }

    fn process(&mut self) -> Result<()> {
        if let Some(spilling_meta) = self.spilling_meta.take() {
            if let AggregateMeta::Spilling(payload) = spilling_meta {
                let bucket = payload.bucket;
                let data_block = serialize_group_by(&self.method, payload)?;
                let columns = get_columns(data_block);

                let mut total_size = 0;
                let mut columns_data = Vec::with_capacity(columns.len());
                for column in columns.into_iter() {
                    let column = column.value.as_column().unwrap();
                    let column_data = serialize_column(column);
                    total_size += column_data.len();
                    columns_data.push(column_data);
                }

                self.writing_data_block = Some((bucket, total_size, columns_data));
                return Ok(());
            }

            return Err(ErrorCode::Internal(""));
        }

        Ok(())
    }

    async fn async_process(&mut self) -> Result<()> {
        if let Some((bucket, total_size, data)) = self.writing_data_block.take() {
            let instant = Instant::now();
            let unique_name = GlobalUniqName::unique();
            let location = format!("{}/{}", self.location_prefix, unique_name);
            let object = self.operator.object(&location);

            // temp code: waiting https://github.com/datafuselabs/opendal/pull/1431
            let mut write_data = Vec::with_capacity(total_size);
            let mut columns_layout = Vec::with_capacity(data.len());

            for data in data.into_iter() {
                columns_layout.push(data.len());
                write_data.extend(data);
            }

            object.write(write_data).await?;
            info!(
                "Write aggregate spill {} successfully, elapsed: {:?}",
                &location,
                instant.elapsed()
            );

            self.spilled_meta = Some(AggregateMeta::<Method, ()>::create_spilled(
                bucket,
                location,
                columns_layout,
            ));
        }

        Ok(())
    }
}

fn get_columns(data_block: DataBlock) -> Vec<BlockEntry> {
    data_block.columns().to_vec()
}
```

### event

与前面几个 `event` 类似，不同的地方在于：

（1）当 `self.input.has_data()` 为 true 时，我们将从 DataBlock 中取出 meta，然后 downcast 成 `AggregateMeta`，检查其类型：（1）如果发现类型是 `AggregateMeta::Spilling`，则我们需要将其 spill 到存储上，于是我们将 downcast 后的结果赋值给 `self.spilling_meta`，等待在 `process` 中处理，返回 `Event::Sync`；（2）其他类型则直接调用 `self.output.push_data(Ok(data_block))` push 出去，然后返回 `Event::NeedConsume`。

（2）如果发现 `self.spilled_meta` 有数据，表示这个数据已经被 spill 了，则将这个 meta 包装成一个空的 DataBlock 并 push 出去，返回 `Event::NeedConsume`。

### process

`process` 是对 `self.spilling_meta` 进行处理，将其转变为 `self.writing_data_block`，随后交给 `async_process` spill 到存储上：

（1）首先检查 `self.spilling_meta` 中是否有数据，并获得 spilling_meta 中的 hash table。

（2）将 hash table 序列化为 DataBlock，并取出其中的列 `columns: Vec<BlockEntry>`，然后将每一列序列化为字节 `column_data` 并 push 到 `columns_data` 中。

（3）最后对 `self.writing_data_block` 进行赋值：`self.writing_data_block = Some((bucket, total_size, columns_data));`，等待在 `async_process` 中被 spill 到存储中。

### async_process

将 `self.writing_data_block` spill 到存储中，然后将 spilled 后数据的 `bucket`，`location` 和 `columns_layout` 信息包装成一个 `AggregateMeta::Spilled` 类型的 meta 赋值给 `self.spilled_meta`，等待下一次调用 `event` 发送出去。 

## TransformPartitionBucket

`src/query/service/src/pipelines/processors/transforms/aggregator/transform_partition_bucket.rs`

首先介绍一下 `TransformPartitionBucket`，它的 input 可以有多个，但是 output 只有一个，它的作用是将多个 bucket id 相同的 DataBlock 组成一个 `AggregateMeta::Partitioned` 发送出去。

#### initialize_all_inputs

```rust
impl<Method: HashMethodBounds, V: Copy + Send + Sync + 'static>
    TransformPartitionBucket<Method, V>
{
  	// ...
    fn initialize_all_inputs(&mut self) -> Result<bool> {
        self.initialized_all_inputs = true;

        for index in 0..self.inputs.len() {
            if self.inputs[index].port.is_finished() {
                continue;
            }

            // We pull the first unsplitted data block
            if self.inputs[index].bucket > SINGLE_LEVEL_BUCKET_NUM {
                continue;
            }

            if !self.inputs[index].port.has_data() {
                self.inputs[index].port.set_need_data();
                self.initialized_all_inputs = false;
                continue;
            }

            let data_block = self.inputs[index].port.pull_data().unwrap()?;
            self.inputs[index].bucket = self.add_bucket(data_block);

            if self.inputs[index].bucket <= SINGLE_LEVEL_BUCKET_NUM {
                self.inputs[index].port.set_need_data();
                self.initialized_all_inputs = false;
            }
        }

        Ok(self.initialized_all_inputs)
    }
  	// ...
}
```

首先我们先看一下 `initialize_all_inputs` 这个函数，每次调用 event 的时候，我们都会首先：

```rust
// We pull the first unsplitted data block
if !self.initialized_all_inputs && !self.initialize_all_inputs()? {
    return Ok(Event::NeedData);
}
```

它的作用是将 unsplitted data block，即 bucket id 为 -1 的 block 全 pull 过来，我们先回顾一下 `TransformPartitionBucket` 的上游的上游，即 `AccumulatingTransformer`，在 `AccumulatingTransformer` 中，我们如果 hash table 过大，我们会将其 spill 到存储上，而如果没有 spill 的话，会在 on_finish 的时候返回 bucket id 为 -1 的 DataBlock，而一旦有 spill，则不会有 bucket id 为 -1 的 DataBlock 被 push 到下游，上面这段代码利用了这一特点保证了 bucket id 为 -1 的 DataBlock 全都 pull 过来后，才会向下，执行，否则会一直返回 `Event::NeedData`。

### event

（1）如果 `self.output.is_finished()` 为 true，调用每个 input 的 `finish` 并清空 `buckets_blocks`。

（2）利用 `!self.buckets_blocks.is_empty() && !self.unsplitted_blocks.is_empty()` 将所有的 unsplitted data block 全都 pull 过来后才会向下执行。

（3）如果 `!self.buckets_blocks.is_empty() && !self.unsplitted_blocks.is_empty()` 为 true，表示在 pull unsplitted data 的时候把 bucket id 不为 -1 的也 pull 过来了，这时候返回 `Event::Sync`，进而在下次调用 `process` 的时候将 bucket id 为 -1 的 DataBlock partition 为多个 bucket id 不为 -1 的 DataBlock。

（4）如果 `!self.buckets_blocks.is_empty() && !self.unsplitted_blocks.is_empty()` 为 false，表示 pull 过来的都是 bucket id 为 -1 的 DataBlock 或者 bucket id 为 -1 的 DataBlock 已经被 partition 为 bucket id 不为 -1 的 DataBlock 了。这时候我们首先调用 `try_push_data_block` 来 push bucket id 为 -1 的 DataBlock，bucket id 不为 -1 由于代码中 `self.pushing_bucket < self.working_bucket` 的限制还不能被 push。

（5）然后就是一个 loop 循环，具体做的事情就是 bucket id 从 0 开始，等 bucket id 为 0 的都 pull 过来了，再 pull bucket id 为 1 的，以此类推，一旦某个所有的 input 都 finish 了或者某个 input 的数据没准备好，则 break；

（6）如果之前那次 push 有数据被 push 了或本次 push 返回 true，则返回 `Event::NeedConsume`。

（7）从 `buckets_blocks` 中 pop first，调用 `convert_blocks` 将多个 bucket id 相同的 DataBlock 组成一个 `AggregateMeta::Partitioned` 发送出去。（在 `try_push_two_level` 中， `self.pushing_bucket` 是递增不会退的，因此可能 bucket id 小的 DataBlock 不会在 `try_push_two_level` 中被 push 出去，而会在这里被 push 出去。

#### add_bucket

```rust
impl<Method: HashMethodBounds, V: Copy + Send + Sync + 'static>
    TransformPartitionBucket<Method, V>
{
  	// ...
    fn add_bucket(&mut self, data_block: DataBlock) -> isize {
        if let Some(block_meta) = data_block.get_meta() {
            if let Some(block_meta) = AggregateMeta::<Method, V>::downcast_ref_from(block_meta) {
                let (bucket, res) = match block_meta {
                    AggregateMeta::Spilling(_) => unreachable!(),
                    AggregateMeta::Partitioned { .. } => unreachable!(),
                    AggregateMeta::Spilled(payload) => (payload.bucket, SINGLE_LEVEL_BUCKET_NUM),
                    AggregateMeta::Serialized(payload) => (payload.bucket, payload.bucket),
                    AggregateMeta::HashTable(payload) => (payload.bucket, payload.bucket),
                };

                if bucket > SINGLE_LEVEL_BUCKET_NUM {
                    match self.buckets_blocks.entry(bucket) {
                        Entry::Vacant(v) => {
                            v.insert(vec![data_block]);
                        }
                        Entry::Occupied(mut v) => {
                            v.get_mut().push(data_block);
                        }
                    };

                    return res;
                }
            }
        }

        self.unsplitted_blocks.push(data_block);
        SINGLE_LEVEL_BUCKET_NUM
    }
  	// ...
}
```

将一个 `DataBlock` 加到 `unsplitted_blocks` 或者 `buckets_blocks` 中，可以看到，bucket id 为 -1 的 DataBlock 都会被 push 到 `unsplitted_blocks` 中。

### Process

```rust
impl<Method: HashMethodBounds, V: Copy + Send + Sync + 'static>
    TransformPartitionBucket<Method, V>
{
    fn process(&mut self) -> Result<()> {
        let block_meta = self
            .unsplitted_blocks
            .pop()
            .and_then(|mut block| block.take_meta())
            .and_then(AggregateMeta::<Method, V>::downcast_from);

        match block_meta {
            None => Err(ErrorCode::Internal(
                "Internal error, TransformPartitionBucket only recv AggregateMeta.",
            )),
            Some(agg_block_meta) => {
                let data_blocks = match agg_block_meta {
                    AggregateMeta::Spilled(_) => unreachable!(),
                    AggregateMeta::Spilling(_) => unreachable!(),
                    AggregateMeta::Partitioned { .. } => unreachable!(),
                    AggregateMeta::Serialized(payload) => self.partition_block(payload)?,
                    AggregateMeta::HashTable(payload) => self.partition_hashtable(payload)?,
                };

                for (bucket, block) in data_blocks.into_iter().enumerate() {
                    if let Some(data_block) = block {
                        match self.buckets_blocks.entry(bucket as isize) {
                            Entry::Vacant(v) => {
                                v.insert(vec![data_block]);
                            }
                            Entry::Occupied(mut v) => {
                                v.get_mut().push(data_block);
                            }
                        };
                    }
                }

                Ok(())
            }
        }
    }
}
```

可以看到 process 是对 bucket id 为 -1 的 DataBlock 调用 `partition_block` 或 `partition_hashtable` 进行 partition 从而得到 `data_blocks`，然后将 `data_blocks` 插入到 `buckets_blocks` 中。

## TransformSpillReader

`src/query/service/src/pipelines/processors/transforms/aggregator/serde/transform_spill_reader.rs`

如果 DataBlock 不是 `Spilled` 类型，则直接 push 到下游，否则需要进行一些列处理：

`TransformSpillReader` 的处理是围绕着三个成员变量展开的：`reading_meta`，`deserializing_meta` 和 `deserialized_meta`：

+ `reading_meta`：上游传来的 `AggregateMeta::Spilled` 类型的 DataBlock，将它转交给 `self.reading_meta` 然后返回 `Event::Async`，在后面会调用 `async_process` 对其进行异步读取。
+ `deserializing_meta`：异步线程会调用 `async_process` 对 `reading_meta` 进行处理：按照 `reading_meta` 中的信息读取存储，并将读到的内容存到的 `self.deserializing_meta` 中。在后续调用 `event` 时如果发现 `self.deserializing_meta.is_some()` 为 true，则返回 `Event::Sync` 来让线程调用 `process` 进行反序列化。
+ `deserialized_meta`：将 `deserializing_meta` 中的数据进行反序列化，对于 `AggregateMeta::Spilled` 类型的 meta，我们将其分序列化为 `AggregateMeta::Serialized`。而对于 `AggregateMeta::Partitioned` 类型的 meta，我们将其中每个 meta 都反序列化为 `AggregateMeta::Serialized`，然后组成一个 `AggregateMeta::Partitioned`。最终我们将反序列化后的结果转交给 `deserialized_meta`，让它在下次 `event` 时被 push 出去。

## BlockMetaTransformer

```rust
// src/query/pipeline/transforms/src/processors/transforms/transform.rs
#[async_trait::async_trait]
impl<B: BlockMetaInfo, T: BlockMetaTransform<B>> Processor for BlockMetaTransformer<B, T> {
    fn name(&self) -> String {
        String::from(T::NAME)
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn event(&mut self) -> Result<Event> {
        if !self.called_on_start {
            return Ok(Event::Sync);
        }

        match self.output.is_finished() {
            true => self.finish_input(),
            false if !self.output.can_push() => self.not_need_data(),
            false => match self.output_data.take() {
                None if self.input_data.is_some() => Ok(Event::Sync),
                None => self.pull_data(),
                Some(data) => {
                    self.output.push_data(Ok(data));
                    Ok(Event::NeedConsume)
                }
            },
        }
    }

    fn process(&mut self) -> Result<()> {
        if !self.called_on_start {
            self.called_on_start = true;
            self.transform.on_start()?;
            return Ok(());
        }

        if let Some(mut data_block) = self.input_data.take() {
            debug_assert!(data_block.is_empty());
            if let Some(block_meta) = data_block.take_meta() {
                if let Some(block_meta) = B::downcast_from(block_meta) {
                    let data_block = self.transform.transform(block_meta)?;
                    self.output_data = Some(data_block);
                }
            }

            return Ok(());
        }

        if !self.called_on_finish {
            self.called_on_finish = true;
            self.transform.on_finish()?;
        }

        Ok(())
    }
}
```


### process

如果 `input_data` 有数据的话，将 block_meta downcast 成实现 `trait BlockMetaInfo` 的某种 meta，例如 `AggregateMeta`，然后调用 `self.transform.transform(block_meta)?` 将 meta 转换 column 不为空的 DataBlock，然后将其转交给 `self.output_data` 等待下一次 event 时被 push 出去。

## CompoundBlockOperator

调用链：`Processor` 会包装一个 `Transformer`，`Transformer` 里面有一个 `transform` 成员，这个成员就是 `BlockOperator` 类型，调用 `Processor` 的 `process` 会调用 `Transformer` 的 `self.transform.transform` 进而调用 `BlockOperator` 的 `execute` 函数将 DataBlock transform 成另外的格式（例如 projection）

BlockOperator 有四种类型：

```rust
// src/query/sql/src/evaluator/block_operator.rs
/// `BlockOperator` takes a `DataBlock` as input and produces a `DataBlock` as output.
#[derive(Clone)]
pub enum BlockOperator {
    /// Batch mode of map which merges map operators into one.
    Map { exprs: Vec<Expr> },

    /// Filter the input `DataBlock` with the predicate `eval`.
    Filter { expr: Expr },

    /// Reorganize the input `DataBlock` with `projection`.
    Project { projection: Vec<FieldIndex> },

    /// Unnest certain fields of the input `DataBlock`.
    Unnest { fields: Vec<usize> },
}
```

execute 函数如下：

```rust
impl BlockOperator {
    pub fn execute(&self, func_ctx: &FunctionContext, mut input: DataBlock) -> Result<DataBlock> {
        match self {
            BlockOperator::Map { exprs } => {
                for expr in exprs {
                    let evaluator = Evaluator::new(&input, *func_ctx, &BUILTIN_FUNCTIONS);
                    let result = evaluator.run(expr)?;
                    let col = BlockEntry {
                        data_type: expr.data_type().clone(),
                        value: result,
                    };
                    input.add_column(col);
                }
                Ok(input)
            }

            BlockOperator::Filter { expr } => {
                assert_eq!(expr.data_type(), &DataType::Boolean);

                let evaluator = Evaluator::new(&input, *func_ctx, &BUILTIN_FUNCTIONS);
                let filter = evaluator.run(expr)?.try_downcast::<BooleanType>().unwrap();
                input.filter_boolean_value(&filter)
            }

            BlockOperator::Project { projection } => {
                let mut result = DataBlock::new(vec![], input.num_rows());
                for index in projection {
                    result.add_column(input.get_by_offset(*index).clone());
                }
                Ok(result)
            }

            BlockOperator::Unnest { fields } => {
                let num_rows = input.num_rows();
                let mut unnest_columns = Vec::with_capacity(fields.len());
                for field in fields {
                    let col = input.get_by_offset(*field);
                    let array_col = match &col.value {
                        Value::Scalar(Scalar::Array(col)) => {
                            Box::new(ArrayColumnBuilder::<AnyType>::repeat(col, num_rows).build())
                        }
                        Value::Column(Column::Array(col)) => col.clone(),
                        _ => {
                            return Err(ErrorCode::Internal(
                                "Unnest can only be applied to array types.",
                            ));
                        }
                    };
                    unnest_columns.push((*field, array_col));
                }
                Self::fit_unnest(input, unnest_columns)
            }
        }
    }
}
```

至此，一条 SQL 的 pipeline 就执行完毕了。
