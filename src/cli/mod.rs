pub mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(name = "lst", about = "Personal lists & notes app")]
#[clap(version, author)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,

    /// Output in JSON format
    #[clap(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all lists or show contents of a specific list
    #[clap(name = "ls")]
    ListLists {
        /// Name of the list to show (optional)
        list: Option<String>,
    },

    /// Add an item to a list
    #[clap(name = "add")]
    Add {
        /// Name of the list
        list: String,
        /// Text of the item(s) to add (comma-separated for multiple items)
        text: String,
    },

    /// Mark an item as done
    #[clap(name = "done")]
    Done {
        /// Name of the list
        list: String,
        /// Target item to mark as done (anchor, text, or index)
        target: String,
    },

    /// Read items from stdin and add them to a list
    #[clap(name = "pipe")]
    Pipe {
        /// Name of the list
        list: String,
    },

    /// Commands for managing notes
    #[clap(subcommand, name = "note")]
    Note(NoteCommands),

    /// Commands for managing images
    #[clap(subcommand, name = "img")]
    Image(ImageCommands),

    /// Daily list commands (add, done, or display)
    #[clap(name = "dl")]
    Dl {
        #[clap(subcommand)]
        cmd: Option<DlCmd>,
    },

    /// Daily note: create or open today's note
    #[clap(name = "dn")]
    Dn,
}

#[derive(Subcommand)]
pub enum NoteCommands {
    /// Create a new note
    #[clap(name = "new")]
    New {
        /// Title of the note
        title: String,
    },

    /// Append text to a note (create if it doesn't exist)
    #[clap(name = "add")]
    Add {
        /// Title of the note
        title: String,
        /// Text to append to the note
        text: String,
    },

    /// Open a note in the default editor
    #[clap(name = "open")]
    Open {
        /// Title of the note
        title: String,
    },

    /// Delete a note
    #[clap(name = "rm")]
    Remove {
        /// Name of the list
        title: String,
    },

    /// List all notes
    #[clap(name = "ls")]
    ListNotes {},
}

#[derive(Subcommand)]
pub enum ImageCommands {
    /// Add an image to a document
    #[clap(name = "add")]
    Add {
        /// Path to the image file
        file: String,
        /// Document to add the image to
        #[clap(long)]
        to: String,
        /// Caption for the image
        #[clap(long)]
        caption: Option<String>,
    },

    /// Paste image from clipboard
    #[clap(name = "paste")]
    Paste {
        /// Document to add the image to
        #[clap(long)]
        to: Option<String>,
        /// Caption for the image
        #[clap(long)]
        caption: Option<String>,
        /// Output for clipboard
        #[clap(long)]
        clipboard: bool,
    },

    /// List images in a document
    #[clap(name = "list")]
    List {
        /// Document to list images from
        document: String,
    },

    /// Remove an image reference from a document
    #[clap(name = "rm")]
    Remove {
        /// Document containing the image
        document: String,
        /// Hash of the image to remove
        hash: String,
    },
}

/// Subcommands for daily list
#[derive(Subcommand)]
pub enum DlCmd {
    /// Add item to today's daily list
    #[clap(name = "add")]
    Add {
        /// Text of the item to add
        item: String,
    },

    /// Mark an item as done in today's daily list
    #[clap(name = "done")]
    Done {
        /// Target item to mark as done (anchor, text, or index)
        item: String,
    },
}
