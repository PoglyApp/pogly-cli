use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "pogly-cli", disable_version_flag = true)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Args)]
pub struct GlobalArgs {
    /// Overlay profile nickname (defaults to the configured default overlay)
    #[arg(long, global = true)]
    pub overlay: Option<String>,
    /// Print raw JSON responses
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Cmd {
    /// Check that the overlay API is reachable
    Ping,
    /// Show the identity and permissions behind the configured token
    Whoami,
    /// Manage overlay profiles used by the other commands
    Overlay(OverlayCmd),
    /// Manage overlay elements
    Elements(ElementsCmd),
    /// Manage element data assets
    Elementdata(ElementDataCmd),
    /// Manage layouts (scenes)
    Layouts(LayoutsCmd),
    /// Manage asset folders
    Folders(FoldersCmd),
    /// Show or manage the installed CLI version
    Version(VersionCmd),
    /// Run the MCP (Model Context Protocol) server
    Mcp,
    /// Run the OSC (Open Sound Control) listener server
    Osc(OscCmd),
}

#[derive(Args)]
pub struct VersionCmd {
    #[command(subcommand)]
    pub cmd: Option<VersionSub>,
}

#[derive(Subcommand)]
pub enum VersionSub {
    /// Install and switch to the latest release
    Upgrade,
    /// List installed versions
    List,
    /// Switch to another installed version (downloads it if missing)
    Use { version: String },
}

#[derive(Args)]
pub struct OverlayCmd {
    #[command(subcommand)]
    pub cmd: OverlaySub,
}

#[derive(Subcommand)]
pub enum OverlaySub {
    /// List configured overlay profiles
    List,
    /// Add an overlay profile
    Add {
        /// Overlay address (64 hex chars), legacy overlay name, or full overlay URL
        target: String,
        /// API token (pgly_...) minted in Pogly under Settings -> API Access
        #[arg(long)]
        token: String,
        /// Profile nickname (defaults to a short form of the overlay address)
        #[arg(long)]
        nickname: Option<String>,
        /// Skip validating the token with a whoami call
        #[arg(long)]
        no_verify: bool,
    },
    /// Update fields on an overlay profile
    Update {
        nickname: String,
        #[arg(long)]
        token: Option<String>,
        /// Overlay address or overlay URL
        #[arg(long)]
        address: Option<String>,
        /// Change the profile nickname
        #[arg(long)]
        rename: Option<String>,
    },
    /// Remove an overlay profile
    Remove { nickname: String },
    /// Set the default overlay profile
    SetDefault { nickname: String },
}

#[derive(Args)]
pub struct ElementsCmd {
    #[command(subcommand)]
    pub cmd: ElementsSub,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum ElementsSub {
    /// List elements
    List {
        /// Fetch a single element by id
        #[arg(long)]
        id: Option<u32>,
        /// Filter by layout id
        #[arg(long)]
        layout: Option<u32>,
    },
    /// Create an element
    #[command(subcommand)]
    Add(AddElement),
    /// Update fields on an element
    Update(UpdateElement),
    /// Delete an element
    Delete { id: u32 },
}

#[derive(Args, Default)]
pub struct CommonElementArgs {
    #[arg(long)]
    pub x: Option<i64>,
    #[arg(long)]
    pub y: Option<i64>,
    /// CSS transform string (overrides --x/--y)
    #[arg(long)]
    pub transform: Option<String>,
    /// Opacity 0-100
    #[arg(long)]
    pub transparency: Option<i64>,
    /// CSS clip-path string
    #[arg(long)]
    pub clip: Option<String>,
    /// Target layout id (defaults to the active layout)
    #[arg(long)]
    pub layout_id: Option<u32>,
}

#[derive(Subcommand)]
pub enum AddElement {
    /// Add a text element
    Text {
        #[arg(long)]
        text: String,
        /// Font size (default 48)
        #[arg(long)]
        size: Option<i64>,
        /// Text color (default #ffffff)
        #[arg(long)]
        color: Option<String>,
        #[arg(long)]
        font: Option<String>,
        /// Extra CSS applied to the element
        #[arg(long)]
        css: Option<String>,
        #[command(flatten)]
        common: CommonElementArgs,
    },
    /// Add an image element from an asset or URL
    #[command(group(ArgGroup::new("source").required(true).args(["data_id", "url"])))]
    Image {
        /// Existing element data (asset) id
        #[arg(long)]
        data_id: Option<u32>,
        /// Raw image URL (requires --width and --height)
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        width: Option<i64>,
        #[arg(long)]
        height: Option<i64>,
        #[command(flatten)]
        common: CommonElementArgs,
    },
    /// Add a widget element
    #[command(group(ArgGroup::new("source").required(true).args(["data_id", "raw_data"])))]
    Widget {
        /// Existing element data (asset) id
        #[arg(long)]
        data_id: Option<u32>,
        /// Inline widget definition
        #[arg(long)]
        raw_data: Option<String>,
        #[arg(long)]
        width: i64,
        #[arg(long)]
        height: i64,
        #[command(flatten)]
        common: CommonElementArgs,
    },
    /// Add a media (video/audio) element
    Media {
        /// Media source URL (e.g. a YouTube link)
        #[arg(long)]
        source: String,
        /// Volume 0-100 (default 100)
        #[arg(long)]
        volume: Option<i64>,
        #[arg(long)]
        width: Option<i64>,
        #[arg(long)]
        height: Option<i64>,
        /// Start playing immediately
        #[arg(long)]
        autoplay: bool,
        /// Loop playback
        #[arg(long)]
        r#loop: bool,
        /// Start position in seconds
        #[arg(long)]
        timestamp: Option<i64>,
        #[command(flatten)]
        common: CommonElementArgs,
    },
}

#[derive(Args, Default)]
pub struct UpdateElement {
    pub id: u32,
    #[command(flatten)]
    pub common: CommonElementArgs,
    #[arg(long)]
    pub locked: Option<bool>,
    #[arg(long)]
    pub always_loaded: Option<bool>,
    #[arg(long)]
    pub index_lock: Option<bool>,
    /// Text content (text elements)
    #[arg(long)]
    pub text: Option<String>,
    #[arg(long)]
    pub text_size: Option<i64>,
    #[arg(long)]
    pub color: Option<String>,
    #[arg(long)]
    pub font: Option<String>,
    #[arg(long)]
    pub css: Option<String>,
    /// Swap to an asset (image elements)
    #[arg(long)]
    pub image_data_id: Option<u32>,
    /// Swap to a raw image URL (image elements)
    #[arg(long)]
    pub image_url: Option<String>,
    #[arg(long)]
    pub image_width: Option<i64>,
    #[arg(long)]
    pub image_height: Option<i64>,
    /// Swap to an asset (widget elements)
    #[arg(long)]
    pub widget_data_id: Option<u32>,
    /// Replace the inline widget definition (widget elements)
    #[arg(long)]
    pub widget_raw_data: Option<String>,
    #[arg(long)]
    pub widget_width: Option<i64>,
    #[arg(long)]
    pub widget_height: Option<i64>,
    /// Media source URL (media elements)
    #[arg(long)]
    pub media_source: Option<String>,
    #[arg(long)]
    pub media_volume: Option<i64>,
    #[arg(long)]
    pub media_playing: Option<bool>,
    /// Seek position in seconds
    #[arg(long)]
    pub media_timestamp: Option<i64>,
    #[arg(long)]
    pub media_autoplay: Option<bool>,
    #[arg(long)]
    pub media_loop: Option<bool>,
    #[arg(long)]
    pub media_width: Option<i64>,
    #[arg(long)]
    pub media_height: Option<i64>,
}

#[derive(Args)]
pub struct ElementDataCmd {
    #[command(subcommand)]
    pub cmd: ElementDataSub,
}

#[derive(ValueEnum, Clone, Copy)]
pub enum DataType {
    Image,
    Widget,
    Media,
    Text,
}

impl DataType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataType::Image => "image",
            DataType::Widget => "widget",
            DataType::Media => "media",
            DataType::Text => "text",
        }
    }
}

#[derive(Subcommand)]
pub enum ElementDataSub {
    /// List element data assets
    List {
        /// Fetch a single asset by id
        #[arg(long)]
        id: Option<u32>,
        /// Filter by exact name
        #[arg(long)]
        name: Option<String>,
        /// Filter by folder id
        #[arg(long)]
        folder: Option<u32>,
    },
    /// Create an element data asset
    Add {
        #[arg(long)]
        name: String,
        #[arg(long, value_enum)]
        r#type: DataType,
        /// The URL or asset data string
        #[arg(long)]
        data: String,
        #[arg(long)]
        width: i64,
        #[arg(long)]
        height: i64,
        #[arg(long)]
        folder_id: Option<u32>,
    },
    /// Update fields on an element data asset
    Update {
        id: u32,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        data: Option<String>,
        #[arg(long)]
        width: Option<i64>,
        #[arg(long)]
        height: Option<i64>,
        #[arg(long)]
        folder_id: Option<u32>,
    },
    /// Delete an asset (also deletes elements that reference it)
    #[command(group(ArgGroup::new("target").required(true).args(["id", "name"])))]
    Delete {
        #[arg(long)]
        id: Option<u32>,
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Args)]
pub struct LayoutsCmd {
    #[command(subcommand)]
    pub cmd: LayoutsSub,
}

#[derive(Subcommand)]
pub enum LayoutsSub {
    /// List layouts
    List,
    /// Create a layout
    Add {
        #[arg(long)]
        name: String,
        /// Make it the active layout immediately
        #[arg(long)]
        active: bool,
    },
    /// Duplicate a layout and its elements
    Duplicate { id: u32 },
    /// Rename a layout
    Rename {
        id: u32,
        #[arg(long)]
        name: String,
    },
    /// Delete a layout (its elements are deleted unless preserved)
    Delete {
        id: u32,
        /// Move the layout's elements instead of deleting them
        #[arg(long)]
        preserve_elements: bool,
        /// Layout to move preserved elements to (default 1)
        #[arg(long)]
        preserve_layout_id: Option<u32>,
    },
    /// Set the active layout (scene switch)
    #[command(group(ArgGroup::new("target").required(true).args(["id", "name"])))]
    SetActive {
        #[arg(long)]
        id: Option<u32>,
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Args)]
pub struct FoldersCmd {
    #[command(subcommand)]
    pub cmd: FoldersSub,
}

#[derive(Subcommand)]
pub enum FoldersSub {
    /// List folders
    List,
    /// Create a folder
    Add {
        #[arg(long)]
        name: String,
        #[arg(long)]
        icon: Option<String>,
    },
    /// Update a folder's name and/or icon
    Update {
        id: u32,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        icon: Option<String>,
    },
    /// Delete a folder (its assets are kept unless --no-preserve-elements)
    Delete {
        id: u32,
        /// Also delete the folder's assets
        #[arg(long)]
        no_preserve_elements: bool,
    },
}

#[derive(Args)]
pub struct OscCmd {
    /// Host interface to bind to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// UDP port to listen on
    #[arg(long, default_value = "9000")]
    pub port: u16,
}

