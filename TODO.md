- cmd exec:
    [x] repeat on cmd failure
    [x] disable profile on cmd failure
    - exec_end: cmd to execute when process vanishes
      - same as not_seen condition !?
- conditions:
    - resource conditions: ram, cpu, net usage ...
