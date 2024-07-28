[ ] use state machine
[ ] match multiple patterns
[ ] cmd exec:
    [x] repeat on cmd failure
    [x] disable profile on cmd failure
    [x] exec_end: cmd to execute when matching state ends
[ ] on-off commands 
    [x] should execute cmd once if `run_once` is true
    [ ] should reset the state next time process appears
- conditions:
    - resource conditions: ram, cpu, net usage ...
