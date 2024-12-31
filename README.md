# DiscordGuildSpammer

A simple Discord guild message spammer to celebrate events.


# Usage

1. Create a .env file in the project directory.
2. Populate the file with the following fields (each entry should occupy a separate line):

(*Follow this exact line order.*)

```txt
[Discord Message]   # The message to be spammed
[Guild ID]          # The ID of the Discord guild (server)
[Bot Token 1]       # Bot token for authentication
[Bot Token 2]       # (Optional) Additional bot tokens
[Bot Token ...]     # (Optional) Add more tokens as needed
```
3. Build & run the program with `cargo run`



# What does it do?

This program prompts you to "Begin".

Beginning will send the `[Discord Message]` as quickly as possible through all of the possible guild's text channels. The specific guild is set by specifying the [Guild ID].

If multiple bot tokens are provided, asynchronously spams the messages.



# Quality

This tool was developed quickly and may not meet high standards of code quality. It contains numerous expect() statements and lacks robust error handling.



# Why?

This tool was created to celebrate the New Year with automated Discord messages.
