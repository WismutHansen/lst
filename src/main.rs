mod cli;
mod config;
mod models;
mod storage;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, NoteCommands, PostCommands, ImageCommands};

fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Load configuration
    let _config = config::Config::load()?;
    
    // Process commands
    match &cli.command {
        Commands::ListLists { list } => {
            if let Some(list_name) = list {
                cli::commands::display_list(list_name, cli.json)?;
            } else {
                cli::commands::list_lists(cli.json)?;
            }
        },
        Commands::Add { list, text } => {
            cli::commands::add_item(list, text, cli.json)?;
        },
        Commands::Done { list, target } => {
            cli::commands::mark_done(list, target, cli.json)?;
        },
        Commands::Pipe { list } => {
            cli::commands::pipe(list, cli.json)?;
        },
        Commands::Note(note_cmd) => {
            match note_cmd {
                NoteCommands::New { title } => {
                    cli::commands::note_new(title)?;
                },
                NoteCommands::Open { title } => {
                    cli::commands::note_open(title)?;
                },
            }
        },
        Commands::Post(post_cmd) => {
            match post_cmd {
                PostCommands::New { title: _ } => {
                    eprintln!("Post commands not implemented yet");
                },
                PostCommands::List => {
                    eprintln!("Post commands not implemented yet");
                },
                PostCommands::Publish { slug: _ } => {
                    eprintln!("Post commands not implemented yet");
                },
            }
        },
        Commands::Image(img_cmd) => {
            match img_cmd {
                ImageCommands::Add { file: _, to: _, caption: _ } => {
                    eprintln!("Image commands not implemented yet");
                },
                ImageCommands::Paste { to: _, caption: _, clipboard: _ } => {
                    eprintln!("Image commands not implemented yet");
                },
                ImageCommands::List { document: _ } => {
                    eprintln!("Image commands not implemented yet");
                },
                ImageCommands::Remove { document: _, hash: _ } => {
                    eprintln!("Image commands not implemented yet");
                },
            }
        },
    }
    
    Ok(())
}
