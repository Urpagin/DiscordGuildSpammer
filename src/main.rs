use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use rand::Rng;
use serenity::all::{ChannelType, CreateMessage, GuildChannel, GuildId, Http};
use serenity::prelude::*;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::sleep;
use tokio::{
    io,
    sync::broadcast::{Receiver, Sender},
};

/// Returns a random index
fn get_random_index(length: usize) -> usize {
    let mut rng = rand::thread_rng();

    // Generates a number in the range [0, length[
    rng.gen_range(0..length)
}

/// A singular rotation mechanism for all bot instances.
///
/// # Panics
/// If the provided `rotated_idx` is greater than the length of the slice of channels.
fn get_rotated_channel<'a>(
    text_channels: &'a [GuildChannel],
    rotated_idx: &mut usize,
) -> &'a GuildChannel {
    if text_channels.is_empty() {
        panic!("No text channels available for rotation!");
    }

    // Update the rotated index
    *rotated_idx = (*rotated_idx + 1) % text_channels.len();

    // Return channel
    &text_channels[*rotated_idx]
}

/// Infinite loop that sends the messages to all channels.
/// This function can only be called once.
async fn spam(text_channels: Vec<GuildChannel>, message: CreateMessage, http: Arc<Http>) {
    let channels = text_channels.clone();
    tokio::spawn(async move {
        let mut rotated_idx: usize = get_random_index(channels.len());
        loop {
            // Start with a random channel
            let channel = get_rotated_channel(&channels, &mut rotated_idx);
            if let Err(why) = channel.send_message(&http, message.clone()).await {
                println!("Failed to send message: {why}");
            }
        }
    });
}

/// Starts a Discord bot
async fn start_instance(
    token: &str,
    guild_id: &GuildId,
    intents: &GatewayIntents,
    message: CreateMessage,
    mut rx: Receiver<BroadcastCommand>,
) {
    let mut client = Client::builder(token, *intents)
        .await
        .expect("Err creating client");

    let http = Arc::clone(&client.http);

    let shard_manager = Arc::clone(&client.shard_manager);

    tokio::spawn(async move {
        if let Err(why) = client.start().await {
            println!("Client error: {why:?}");
        }
    });

    let text_channels = guild_id
        .channels(&http)
        .await
        .expect("Failed to fetch channels")
        .values()
        .filter(|channel| channel.kind == ChannelType::Text)
        .cloned()
        .collect::<Vec<_>>();

    println!("Fetched {} text channels", text_channels.len());
    let mut begin_flag: bool = false;

    while let Ok(cmd) = rx.recv().await {
        match cmd {
            BroadcastCommand::Begin => {
                if begin_flag {
                    println!("Already spamming!");
                    continue;
                }
                begin_flag = true;
                spam(text_channels.to_vec(), message.clone(), http.clone()).await;
            }
            BroadcastCommand::Die => {
                shard_manager.shutdown_all().await;
                return;
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BroadcastCommand {
    /// Begins the spam
    Begin,
    /// Terminates the Discord client.
    Die,
}

/// Reads from stdin and broadcasts the command.
async fn user_input_command(tx: Sender<BroadcastCommand>) {
    let mut input = String::new();

    loop {
        println!("Select command:\n[1]: Send messages\n[2]: Disconnect all bots & exit");

        input.clear();

        // Read user input
        io::BufReader::new(io::stdin())
            .read_line(&mut input)
            .await
            .expect("Failed to read input.");

        input = input.trim().to_lowercase();
        let cmd: BroadcastCommand = match input.as_str() {
            "1" => BroadcastCommand::Begin,
            "2" => BroadcastCommand::Die,
            _ => {
                println!("Invalid command. Please retry.");
                continue;
            }
        };

        tx.send(cmd).expect("Failed to broadcast command.");
        println!("Successfully broadcast command!");
    }
}

/// Runs infinitely and exit the app when Die signal received.
async fn await_death(mut rx: Receiver<BroadcastCommand>) -> ! {
    loop {
        if let Ok(cmd) = rx.recv().await {
            if cmd == BroadcastCommand::Die {
                break;
            }
        }
    }

    // Necessary to stop the stdin. Just returning would mean there still is a stdin input in
    // progress. So, we need to use exit().
    exit(0);
}

/// Reads Discord bot tokens from the ".env" file.
/// The contents of the .env file should be as follows,
/// sequentially separated by lines:
/// [message]
/// [GuildID]
/// [Token1]
/// [Token2]
/// [Token...]
///
/// # Panics
/// If malformed .env file or any other error.
async fn read_program_info() -> ProgramInfo {
    let file = File::open(".env")
        .await
        .expect("Failed to open '.env' file");

    let reader = BufReader::new(file);
    let mut lines_stream = reader.lines(); // Create the lines stream

    let mut lines = Vec::new();
    while let Some(line) = lines_stream
        .next_line()
        .await
        .expect("Failed to read next line")
    {
        lines.push(line.trim().to_string());
    }

    if lines.len() < 3 {
        panic!("Missing lines in .env file. From top to bottom:\n[message]\n[GuildID]\n[Token1]\n[TokenX]");
    }

    ProgramInfo {
        guild_id: GuildId::from_str(lines[1].as_str()).expect("Failed to parse GuildID"),
        message: CreateMessage::new().content(lines[0].clone()),
        tokens: lines[2..].to_vec(),
    }
}

#[derive(Debug, Clone)]
struct ProgramInfo {
    guild_id: GuildId,
    message: CreateMessage,
    tokens: Vec<String>,
}

// TODO: Rewrite this hell.
// TODO: (It's not that bad, only that I'm cloning too much)

#[tokio::main]
async fn main() {
    let program_info = read_program_info().await;

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES;

    let (tx, _rx) = tokio::sync::broadcast::channel::<BroadcastCommand>(10);

    let tx_new = tx.clone();
    let rx_new = tx.clone().subscribe();

    tokio::task::spawn(async move {
        user_input_command(tx_new).await;
    });

    for token in &program_info.tokens {
        let rx_new = tx.subscribe();
        let info_new = program_info.clone();
        let token = token.clone();
        tokio::task::spawn(async move {
            start_instance(
                &token,
                &program_info.guild_id,
                &intents,
                info_new.message,
                rx_new,
            )
            .await;
        });
    }

    // Idle infinite work to keep the program from exiting.
    await_death(rx_new).await;
}
