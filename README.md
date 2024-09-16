 # PSWatch 

pswatch is a minimalist process monitoring and task scheduler that allows you to
watch system processes and run custom commands when specific conditions or
patterns are matched. 

**Features**
- Process Matching: match running processes by substring or regex patterns in name, exe path or the entire command line.
- Define conditions and actions. 
- Execute actions when conditions are met on the matched processes.
- Create multiple profiles for complex conditions and action sets
- Systemd `notify` process type integration.

## Installation

### From source

```sh
git clone https://github.com/your-username/pswatch.git
cd pswatch
cargo build --release
```

The binary will be located in `target/release/pswatch`.

## Usage

Pswatch requires a `TOML` based configuration file. By default it uses the
config file under $XDG_CONFIG_DIR/pswatch/config.toml or the one provided as
parameter.

```sh
./pswatch -c /path/to/config.toml
```

The program will watch system processes and execute commands based on the
patterns defined in the configuration file.

## Configuration File

pswatch's behavior is configured using a TOML-formatted configuration file.
The file should contain a list of `profiles`, each containing a `pattern` (the
process name to match), a `regex` flag (set to `true` if the pattern is a
regular expression), and a list of `commands`.

Each command contains a condition (either `seen` or `not_seen` with a duration)
and an array of shell commands (`exec`) to execute when the condition is met. An
optional `run_once` flag can be set to run the command only once per process
detection.

Here's an example configuration file:

```toml
[[profiles]]
matching = { name = "foo" }

# command 1
[[profiles.commands]]
condition = {seen = "5s"}
exec = ["sh", "-c", "notify-end 'foo action'"]

# command 2 
[[profiles.commands]]
condition = { not_seen = "60s" }
exec = ["sh", "-c", "notify-end 'where is foo ?'"]
run_once = true
```

## Examples with Multiple Profiles

You can use multiple profiles within a single configuration file to monitor
different processes and execute commands for matched conditions. Here's an
example configuration that uses two profiles:

```toml
[[profiles]]
pattern = "bar"
matching = { name = "bar" }

[[profiles.commands]]
condition = {not_seen = "5s"}
exec = ["sh", "-c", "notify-send 'bar not seen!'"]

[[profiles]]
matching = { exe_path = ".*baz$", regex = true}

[[profiles.commands]]
condition = {seen = "10s"}
exec = ["sh", "-c", "notify-end '/baz action !'"]
run_once = true # run the command only once when a match is triggered
```

In this example, pswatch will watch for two processes: "bar" and "baz". It
matches `bar` by process name and `.*baz$` by regex. When "bar" is not seen for
5 seconds, it will execute the `exec` action. When "baz" (a regular expression)
is detected, it will execute the corresponding `exec` after a delay of 10 seconds.
The command for "baz" will be run only once per process detection.

## Example Scenarios

1. **Execute a command when a specific process is seen for a certain duration**
   - Define a watch with the desired process name and use `{seen = "duration"}` to specify that the command should be executed when the process has been running for a specified duration (e.g., "5s").

2. **Execute a command when a specific process is not seen for a certain duration**
   - Define a watch with the desired process name and use `{not_seen = "duration"}` to specify that the command should be executed when the process has been absent for a specified duration (e.g., "5s").

3. **Execute multiple commands based on different conditions**
   - Define multiple watch configurations in the same TOML file and specify separate `condition` and `exec` settings for each. pswatch will monitor all configured profiles and execute their respective commands when appropriate.

## Troubleshooting

You can enable more verbose output using the `-d` flag or the `RUST_LOG=debug`
environment variable.

## Contributing

Contributions are welcome! If you'd like to contribute to pswatch, please
follow these steps:

1. Fork the repository on GitHub.
2. Clone your fork to your local machine: `git clone
   https://github.com/your-username/pswatch.git`.
   3. Create a new branch for your changes: `git checkout -b my-feature`.
   4. Make your changes and commit them with descriptive messages: `git commit
      -am 'Add some feature'`.
      5. Push your branch to your GitHub fork: `git push origin my-feature`.
      6. Submit a pull request from your GitHub fork to the main repository.

## License

pswatch is licensed under the AGPLv3 License. See [LICENSE](LICENSE) for more
details.


