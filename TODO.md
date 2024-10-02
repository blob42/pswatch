[ ] cli commands: manage, add, delete ...
[ ] process detection helpers
   - help user target specific process
   - help user generate profile 
[ ] profile names
[x] match multiple patterns: handled by regex
[x] cmd exec:
    [x] repeat on cmd failure
    [x] disable profile on cmd failure
    [x] exec_end: cmd to execute when matching state ends
[ ] on-off commands 
    [x] should execute cmd once if `run_once` is true
    [ ] should reset the state next time process appears [test]
[ ] use state machine ?
- conditions:
    - resource conditions
        - cpu time
        - cpu %
        - cpu load
        - ram %
        - ram size

