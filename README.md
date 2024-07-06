 # PSwatch README

 This is a program that allows you to watch system processes and run custom
 commands when specific patterns are matched. It's written in Rust for better
 performance and safety. This README will guide you through the usage of the
 program, as well as provide examples of using multiple watches within the same
 configuration file.

## Table of Contents

1. [Installation](#installation)
2. [Usage](#usage)
3. [Configuration File](#configuration-file)
4. [Examples with Multiple Watches](#examples-with-multiple-watches)
5. [Troubleshooting](#troubleshooting)
6. [Contributing](#contributing)
7. [License](#license)

## Installation

To install pswatch, clone the repository from GitHub and build it using
Cargo:

```sh
git clone https://github.com/your-username/pswatch.git
cd pswatch
cargo build --release
```

The binary will be located in `target/release/pswatch`.

## Usage

To use pswatch, provide the path to a configuration file as an argument:

```sh
./pswatch -c /path/to/config.toml
```

The program will watch system processes and execute commands based on the
patterns defined in the configuration file.

## Configuration File

pswatch's behavior is configured using a TOML-formatted configuration file.
The file should contain a list of `watches`, each containing a `pattern` (the
process name to match), a `regex` flag (set to `true` if the pattern is a
regular expression), and a list of `commands`.

Each command contains a condition (either `seen` or `not_seen` with a duration)
and an array of shell commands (`exec`) to execute when the condition is met. An
optional `run_once` flag can be set to run the command only once per process
detection.

Here's an example configuration file:

```toml
[[watches]]
pattern = "foo"
regex = false

[[watches.commands]]
condition = {seen = "5s"}
exec = ["sh", "-c", "notify-end action!"]
# run_once = false # uncomment to run the command only once per process
detection
```

## Examples with Multiple Watches

You can use multiple watches within a single configuration file to monitor
different processes and execute commands based on their patterns. Here's an
example configuration that uses two watches:

```toml
[[watches]]
pattern = "bar"
regex = false

[[watches.commands]]
condition = {not_seen = "5s"}
exec = ["sh", "-c", "echo not seen!"]

[[watches]]
pattern = "baz"
regex = true

[[watches.commands]]
condition = {seen = "10s"}
exec = ["sh", "-c", "say 'baz detected!'"]
run_once = true # run the command only once per process detection
```

In this example, pswatch will watch for two processes: "bar" and "baz". When
"bar" is not seen for 5 seconds, it will execute `echo not seen!`. When "baz" (a
regular expression) is detected, it will execute `say 'baz detected!'` after a
delay of 10 seconds. The command for "baz" will be run only once per process
detection.

## Example Scenarios

1. **Execute a command when a specific process is seen for a certain duration**
   - Define a watch with the desired process name and use `{seen = "duration"}` to specify that the command should be executed when the process has been running for a specified duration (e.g., "5s").

2. **Execute a command when a specific process is not seen for a certain duration**
   - Define a watch with the desired process name and use `{not_seen = "duration"}` to specify that the command should be executed when the process has been absent for a specified duration (e.g., "5s").

3. **Execute multiple commands based on different conditions**
   - Define multiple watch configurations in the same TOML file and specify separate `condition` and `exec` settings for each. pswatch will monitor all configured watches and execute their respective commands when appropriate.

## Troubleshooting

If you encounter any issues while using pswatch, please refer to the
[TROUBLESHOOTING.md](TROUBLESHOOTING.md) file in this repository for
troubleshooting tips and solutions.

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

pswatch is licensed under the MIT License. See [LICENSE](LICENSE) for more
details.


