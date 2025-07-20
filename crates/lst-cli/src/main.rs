mod cli;

// Use re-exported modules from lst-core
use lst_cli::{config, models, storage};

use anyhow::Result;
use clap::Parser;
use cli::{AuthCommands, Cli, Commands, GuiCommands, ImageCommands, NoteCommands, ServerCommands};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Configuration is now loaded on first use via a global cache

    // Process commands
    match &cli.command {
        Commands::ListLists { list } => {
            if let Some(list_name) = list {
                cli::commands::display_list(list_name, cli.json)?;
            } else {
                cli::commands::list_lists(cli.json)?;
            }
        }
        Commands::New { list } => {
            cli::commands::new_list(list)?;
        }
        Commands::Add { list, text } => {
            cli::commands::add_item(list, text, cli.json).await?;
        }
        Commands::Open { list } => {
            cli::commands::open_list(list)?;
        }
        Commands::Done { list, target } => {
            cli::commands::mark_done(list, target, cli.json).await?;
        }
        Commands::Undone { list, target } => {
            cli::commands::mark_undone(list, target, cli.json).await?;
        }
        Commands::Reset { list } => {
            cli::commands::reset_list(list, cli.json).await?;
        }
        Commands::Rm { list, target } => {
            cli::commands::remove_item(list, target, cli.json).await?;
        }
        Commands::Wipe { list, force } => {
            cli::commands::wipe_list(list, *force, cli.json)?;
        }
        Commands::Pipe { list } => {
            cli::commands::pipe(list, cli.json)?;
        }
        Commands::Note(note_cmd) => match note_cmd {
            NoteCommands::New { title } => cli::commands::note_new(title).await?,
            NoteCommands::Add { title, text } => {
                cli::commands::note_add(title, text).await?;
            }
            NoteCommands::Open { title } => cli::commands::note_open(title)?,
            NoteCommands::Remove { title } => cli::commands::note_delete(title).await?,
            NoteCommands::ListNotes {} => {
                cli::commands::list_notes(cli.json)?;
            }
            NoteCommands::Tidy => {
                cli::commands::tidy_notes(cli.json)?;
            }
        },
        // Commands::Post(post_cmd) => {
        //     match post_cmd {
        //         PostCommands::New { title: _ } => {
        //             eprintln!("Post commands not implemented yet");
        //         },
        //         PostCommands::List => {
        //             eprintln!("Post commands not implemented yet");
        //         },
        //         PostCommands::Publish { slug: _ } => {
        //             eprintln!("Post commands not implemented yet");
        //         },
        //     }
        // },
        Commands::Dl { cmd } => {
            cli::commands::daily_list(cmd.as_ref(), cli.json).await?;
        }
        Commands::Dn => {
            cli::commands::daily_note(cli.json)?;
        }
        Commands::Sync(sync_cmd) => {
            cli::commands::handle_sync_command(sync_cmd.clone(), cli.json)?;
        }
        Commands::Image(img_cmd) => match img_cmd {
            ImageCommands::Add {
                file: _,
                to: _,
                caption: _,
            } => {
                eprintln!("Image commands not implemented yet");
            }
            ImageCommands::Paste {
                to: _,
                caption: _,
                clipboard: _,
            } => {
                eprintln!("Image commands not implemented yet");
            }
            ImageCommands::List { document: _ } => {
                eprintln!("Image commands not implemented yet");
            }
            ImageCommands::Remove {
                document: _,
                hash: _,
            } => {
                eprintln!("Image commands not implemented yet");
            }
        },
        Commands::Share {
            document,
            writers,
            readers,
        } => {
            cli::commands::share_document(document, writers.as_deref(), readers.as_deref())?;
        }
        Commands::Unshare { document } => {
            cli::commands::unshare_document(document)?;
        }
        Commands::Gui(remote_cmd) => match remote_cmd {
            GuiCommands::Switch { list } => {
                cli::commands::remote_switch_list(list).await?;
            }
            GuiCommands::Message { text } => {
                cli::commands::remote_show_message(text).await?;
            }
        },
        Commands::Tidy => {
            cli::commands::tidy_lists(cli.json)?;
        }
        Commands::Auth(auth_cmd) => match auth_cmd {
            AuthCommands::Request { email, host } => {
                cli::commands::auth_request(email, host.as_deref(), cli.json).await?;
            }
            AuthCommands::Verify { email, token } => {
                cli::commands::auth_verify(email, token, cli.json).await?;
            }
            AuthCommands::Status => {
                cli::commands::auth_status(cli.json)?;
            }
            AuthCommands::Logout => {
                cli::commands::auth_logout(cli.json)?;
            }
        },
        Commands::Server(server_cmd) => match server_cmd {
            ServerCommands::Create {
                kind,
                path,
                content,
            } => {
                cli::commands::server_create(kind, path, content, cli.json).await?;
            }
            ServerCommands::Get { kind, path } => {
                cli::commands::server_get(kind, path, cli.json).await?;
            }
            ServerCommands::Update {
                kind,
                path,
                content,
            } => {
                cli::commands::server_update(kind, path, content, cli.json).await?;
            }
            ServerCommands::Delete { kind, path } => {
                cli::commands::server_delete(kind, path, cli.json).await?;
            }
        },
    }

    Ok(())
}
