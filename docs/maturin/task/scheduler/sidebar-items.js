initSidebarItems({"fn":[["fetch_task_from_scheduler","从任务队列中拿一个任务，返回其TCB。 非阻塞，即如果没有任务可取，则直接返回 None"],["push_task_to_scheduler","向任务队列里插入一个任务"]],"struct":[["GLOBAL_TASK_SCHEDULER","任务调度器。它是全局的，每次只能有一个核访问它 它启动时会自动在队列中插入 ORIGIN_USER_PROC 作为第一个用户程序"],["Scheduler","任务调度器，目前采用 Round-Robin 算法 在 struct 外部会加一个 Mutex 锁"]]});