initSidebarItems({"struct":[["CoreExecutor","A `CoreExecutor` is the most simple executor provided. It runs a single thread, which is responsible for both scheduling the function (registering the timer for the wakeup), and the actual execution. The executor will stop once dropped. The `CoreExecutor` can be cloned to generate a new reference to the same underlying executor. Given the single threaded nature of this executor, tasks are executed sequentially, and a long running task will cause delay in other subsequent executions."],["ThreadPoolExecutor","A `ThreadPoolExecutor` will use one thread for the task scheduling and a thread pool for task execution, allowing multiple tasks to run in parallel."]]});