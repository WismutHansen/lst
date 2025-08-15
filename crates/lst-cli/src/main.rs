mod cli;

// Use re-exported modules from lst-core
use lst_cli::{config, models, storage};

use anyhow::Result;
use clap::Parser;
use cli::{AuthCommands, CategoryCommands, Cli, Commands, GuiCommands, ImageCommands, NoteCommands, ServerCommands, ThemeCommands, UserCommands};

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
        Commands::Add { list, text, category } => {
            cli::commands::add_item(list, text, category.as_deref(), cli.json).await?;
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
        Commands::Category(cat_cmd) => match cat_cmd {
            CategoryCommands::Add { list, name } => {
                cli::commands::category_add(list, name, cli.json).await?;
            }
            CategoryCommands::Move { list, item, category } => {
                cli::commands::category_move(list, item, category, cli.json).await?;
            }
            CategoryCommands::List { list } => {
                cli::commands::category_list(list, cli.json).await?;
            }
            CategoryCommands::Remove { list, name } => {
                cli::commands::category_remove(list, name, cli.json).await?;
            }
        },
        Commands::Auth(auth_cmd) => match auth_cmd {
            AuthCommands::Register { email, host } => {
                cli::commands::auth_register(email, host.as_deref(), cli.json).await?;
            }
            AuthCommands::Login { email, auth_token } => {
                cli::commands::auth_login(email, auth_token, cli.json).await?;
            }
            AuthCommands::Request { email, host } => {
                cli::commands::auth_request(email, host.as_deref(), cli.json).await?;
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
        Commands::Themes(theme_cmd) => match theme_cmd {
            ThemeCommands::List { verbose } => {
                cli::commands::theme_list(*verbose, cli.json)?;
            }
            ThemeCommands::Current => {
                cli::commands::theme_current(cli.json)?;
            }
            ThemeCommands::Apply { theme } => {
                cli::commands::theme_apply(theme, cli.json).await?;
            }
            ThemeCommands::Info { theme } => {
                cli::commands::theme_info(theme, cli.json)?;
            }
            ThemeCommands::Validate { file } => {
                cli::commands::theme_validate(file, cli.json)?;
            }
        },
        Commands::User(user_cmd) => match user_cmd {
            UserCommands::List => {
                cli::commands::user_list(cli.json).await?;
            }
            UserCommands::Create { email, name } => {
                cli::commands::user_create(email, name.as_deref(), cli.json).await?;
            }
            UserCommands::Delete { email, force } => {
                cli::commands::user_delete(email, *force, cli.json).await?;
            }
            UserCommands::Update { email, name, enabled } => {
                cli::commands::user_update(email, name.as_deref(), *enabled, cli.json).await?;
            }
            UserCommands::Info { email } => {
                cli::commands::user_info(email, cli.json).await?;
            }
        },
    }

    Ok(())
}
