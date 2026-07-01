//! A live gallery of glazier components, mirroring shadcn/ui's landing page:
//! a centered hero section above a muted-background grid of showcase cards.
//!
//! Run with: `cargo run --example gallery`
//!
//! Focus mode: we're polishing the showcase one card at a time, so some helpers
//! and `State` fields belong to cards that are temporarily parked.
#![allow(clippy::too_many_lines)] // a gallery is one long enumeration
#![allow(dead_code)] // parked cards' helpers/state kept for re-enabling

use eframe::egui;
use egui::{Color32, RichText, Vec2, Widget as _};
use glazier::{
    accordion::Accordion,
    alert::{self, Alert},
    alert_dialog::AlertDialog,
    attachment::Attachment,
    avatar::Avatar,
    badge::{self, Badge},
    breadcrumb::Breadcrumb,
    bubble::{Align as BubbleAlign, Bubble, BubbleGroup, Variant as BubbleVariant},
    button::{Button, Size, Variant},
    button_group::ButtonGroup,
    calendar::{Calendar, Date},
    card::Card,
    carousel::Carousel,
    chart::{Chart, Kind as ChartKind, Series},
    checkbox::Checkbox,
    collapsible::Collapsible,
    combobox::Combobox,
    command::{Command, CommandGroup, CommandItem},
    context_menu::ContextMenu,
    dialog::{Dialog, Side},
    drawer::Drawer,
    dropdown_menu::DropdownMenu,
    empty::Empty,
    field::Field,
    grid::Grid,
    hover_card::HoverCard,
    icon::Icon,
    input::Input,
    input_group::InputGroup,
    input_otp::{InputOtp, OtpMode},
    item::{self, Item},
    label::Label,
    marker::{self, Marker},
    menubar::{Menubar, MenubarMenu},
    message::{Message, Side as MsgSide},
    message_scroller::{MessageScroller, Position},
    native_select::NativeSelect,
    navigation_menu::{NavItem, NavigationMenu},
    pagination::Pagination,
    popover::Popover,
    progress::Progress,
    radio_group::RadioGroup,
    resizable::Resizable,
    scroll_area::ScrollArea,
    select::Select,
    separator::Separator,
    sheet::Sheet,
    sidebar_menu::SidebarMenu,
    slider::Slider,
    sonner::Toast,
    spinner::Spinner,
    switch::Switch,
    table::{Sizing, Table},
    tabs::Tabs,
    textarea::Textarea,
    toggle::Toggle,
    toggle_group::ToggleGroup,
    tokens::Tokens,
    tooltip::Tooltip,
    typography::{self, Typography},
    Customize as _,
};

/// Raw [lucide](https://lucide.dev) icon markup, fed straight to [`Icon`].
mod lucide {
    /// `search` — magnifier.
    pub const SEARCH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>"#;
    /// `arrow-right`.
    pub const ARROW_RIGHT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="m12 5 7 7-7 7"/></svg>"#;
    /// `chevron-up`.
    pub const CHEVRON_UP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m18 15-6-6-6 6"/></svg>"#;
    /// `chevron-down`.
    pub const CHEVRON_DOWN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;
    /// `x` — close / dismiss.
    pub const X: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>"#;
    /// `contrast` (tabler) — the light/dark theme toggle glyph.
    pub const CONTRAST: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 12m-9 0a9 9 0 1 0 18 0a9 9 0 1 0 -18 0"/><path d="M12 3l0 18"/><path d="M12 9l4.65 -4.65"/><path d="M12 14.3l7.37 -7.37"/><path d="M12 19.6l8.85 -8.85"/></svg>"#;
    /// `lock-keyhole` — the secured-credentials glyph.
    pub const LOCK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>"#;
    /// `circle-alert` — destructive warning glyph.
    pub const CIRCLE_ALERT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/></svg>"#;
    /// `settings` — gear, for the transfer-limit item.
    pub const SETTINGS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>"#;
    /// `repeat` — recurring-payments glyph.
    pub const REPEAT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m17 2 4 4-4 4"/><path d="M3 11v-1a4 4 0 0 1 4-4h14"/><path d="m7 22-4-4 4-4"/><path d="M21 13v1a4 4 0 0 1-4 4H3"/></svg>"#;
    /// `info` — informational alert glyph.
    pub const INFO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>"#;
    /// `chevron-right` — trailing affordance.
    pub const CHEVRON_RIGHT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>"#;
    /// `inbox` — the empty-state glyph.
    pub const INBOX: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="22 12 16 12 14 15 10 15 8 12 2 12"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/></svg>"#;
    /// `check` — the explored/done marker glyph.
    pub const CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>"#;
    /// `git-branch` — the branch-switch marker glyph.
    pub const GIT_BRANCH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="6" x2="6" y1="3" y2="15"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/></svg>"#;

    // --- Sidebar navigation glyphs (the 2×2 navigation card) ----------------
    /// `chart-no-axes-column` — Analytics.
    pub const ANALYTICS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" x2="18" y1="20" y2="10"/><line x1="12" x2="12" y1="20" y2="4"/><line x1="6" x2="6" y1="20" y2="14"/></svg>"#;
    /// `arrow-right-left` — Transactions.
    pub const TRANSACTIONS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m16 3 4 4-4 4"/><path d="M20 7H4"/><path d="m8 21-4-4 4-4"/><path d="M4 17h16"/></svg>"#;
    /// `trending-up` — Investments.
    pub const INVESTMENTS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="22 7 13.5 15.5 8.5 10.5 2 17"/><polyline points="16 7 22 7 22 13"/></svg>"#;
    /// `landmark` — Accounts.
    pub const ACCOUNTS_ICON: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="3" x2="21" y1="22" y2="22"/><line x1="6" x2="6" y1="18" y2="11"/><line x1="10" x2="10" y1="18" y2="11"/><line x1="14" x2="14" y1="18" y2="11"/><line x1="18" x2="18" y1="18" y2="11"/><polygon points="12 2 20 7 4 7"/></svg>"#;
    /// `pie-chart` — Spending.
    pub const SPENDING: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21.21 15.89A10 10 0 1 1 8 2.83"/><path d="M22 12A10 10 0 0 0 12 2v10z"/></svg>"#;
    /// `file-text` — Documents.
    pub const DOCUMENTS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M10 9H8"/><path d="M16 13H8"/><path d="M16 17H8"/></svg>"#;
    /// `wallet` — Budget.
    pub const BUDGET: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 7V4a1 1 0 0 0-1-1H5a2 2 0 0 0 0 4h15a1 1 0 0 1 1 1v4h-3a2 2 0 0 0 0 4h3a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1"/><path d="M3 5v14a2 2 0 0 0 2 2h15a1 1 0 0 0 1-1v-4"/></svg>"#;
    /// `chart-spline` — Reports.
    pub const REPORTS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3v16a2 2 0 0 0 2 2h16"/><path d="M7 16c.5-2 2-7 4-7 2 0 2 5 4 5 1.5 0 3-4 3-4"/></svg>"#;
    /// `target` — Goals.
    pub const GOALS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="6"/><circle cx="12" cy="12" r="2"/></svg>"#;
    /// `calendar` — Calendar.
    pub const CALENDAR: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 2v4"/><path d="M16 2v4"/><rect width="18" height="18" x="3" y="4" rx="2"/><path d="M3 10h18"/></svg>"#;
    /// `circle-help` — Help Center.
    pub const HELP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><path d="M12 17h.01"/></svg>"#;
    /// `book-open` — Docs.
    pub const DOCS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 7v14"/><path d="M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z"/></svg>"#;
    /// `message-circle-question` — Contact Us.
    pub const CONTACT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M7.9 20A9 9 0 1 0 4 16.1L2 22Z"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><path d="M12 17h.01"/></svg>"#;
    /// `square-activity` — Status.
    pub const STATUS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M17 12h-2l-2 5-2-10-2 5H7"/></svg>"#;
    /// `globe` — Community.
    pub const COMMUNITY: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"/><path d="M2 12h20"/></svg>"#;
    /// `user` — Profile.
    pub const PROFILE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>"#;
    /// `credit-card` — Billing.
    pub const BILLING: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="20" height="14" x="2" y="5" rx="2"/><line x1="2" x2="22" y1="10" y2="10"/></svg>"#;
    /// `bell` — Notifications.
    pub const BELL: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.268 21a2 2 0 0 0 3.464 0"/><path d="M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326"/></svg>"#;
    /// `shield` — Security.
    pub const SHIELD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/></svg>"#;
    /// `palette` — Appearance.
    pub const APPEARANCE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="13.5" cy="6.5" r=".5" fill="currentColor"/><circle cx="17.5" cy="10.5" r=".5" fill="currentColor"/><circle cx="8.5" cy="7.5" r=".5" fill="currentColor"/><circle cx="6.5" cy="12.5" r=".5" fill="currentColor"/><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"/></svg>"#;
}

/// Persistent state for the gallery's interactive widgets.
#[allow(clippy::struct_excessive_bools)] // a demo toggling many controls
struct State {
    name: String,
    framework: String,
    notifications: bool,
    marketing: bool,
    terms: bool,
    // Kitchen-sink card.
    search: String,
    message: String,
    fruit: usize,
    email_alerts: bool,
    compact: bool,
    menu_choice: Option<usize>,
    alert_open: bool,
    // Transfer funds card.
    transfer_amount: String,
    from_account: usize,
    to_account: usize,
    // Payout threshold card.
    payout_currency: usize,
    payout_amount: f64,
    payout_notes: String,
    // Q2 dividend income card.
    dividend_dismissed: bool,
    // Account access card.
    access_email: String,
    access_password: String,
    // Navigation (2×2 sidebar-menu) card — active item per quadrant.
    nav_overview: usize,
    nav_planning: usize,
    nav_support: usize,
    nav_account: usize,
    // Collapsible Sidebar shell card.
    sidebar_nav: usize,
    // Controls card.
    volume: f32,
    muted: bool,
    align: usize,
    // Disclosure card.
    active_tab: usize,
    // Overlays card.
    dialog_open: bool,
    dialog_name: String,
    dialog_handle: String,
    sheet_open: bool,
    sheet_side: usize,
    drawer_open: bool,
    drawer_side: usize,
    popover_side: usize,
    // Forms card.
    field_name: String,
    field_username: String,
    field_workspace: String,
    loading: bool,
    context_log: Option<String>,
    // Data card.
    table_page: usize,
    carousel_idx: usize,
    // Date & feedback card.
    selected_date: Option<Date>,
    picker_date: Option<Date>,
    input_date: Option<Date>,
    input_date_text: String,
    picker_time: glazier::Time,
    toast_corner: glazier::sonner::Corner,
    // Conversation (Message + MessageScroller) card.
    chat: Vec<ChatMsg>,
    chat_draft: String,
    chat_seq: usize,
    // Typography card — NativeSelect bound to a heading-size choice.
    type_size: usize,
    // Typography card — Combobox bound to a framework choice.
    combo_framework: usize,
    // Typography card — Combobox over a long list (scroll demo).
    combo_timezone: usize,
    // Command card — the last activated command id (for the status line).
    command_last: Option<String>,
    // Menubar card — the last chosen "Menu › Item" label (for the status line).
    menubar_last: Option<String>,
    // NavigationMenu card — the last clicked link/panel label (for the status line).
    nav_last: Option<String>,
    // Input OTP card — the entered code.
    otp_code: String,
    // Attachment card — the staged files (name, bytes).
    attachments: Vec<(String, u64)>,
    // Chart card — the selected mark (0 bar / 1 line / 2 area).
    chart_kind: usize,
}

/// One message in the conversation showcase card.
#[derive(Clone)]
struct ChatMsg {
    id: usize,
    mine: bool,
    text: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            name: String::new(),
            framework: String::new(),
            notifications: true,
            marketing: false,
            terms: true,
            search: String::new(),
            message: String::new(),
            fruit: 0,
            email_alerts: true,
            compact: true,
            menu_choice: None,
            alert_open: false,
            transfer_amount: "1,200.00".to_owned(),
            from_account: 0,
            to_account: 1,
            payout_currency: 0,
            payout_amount: 2500.0,
            payout_notes: String::new(),
            dividend_dismissed: false,
            access_email: String::new(),
            access_password: String::new(),
            sidebar_nav: 0,
            nav_overview: 0, // Analytics active
            nav_planning: 0,
            nav_support: 0,
            nav_account: 1, // Billing active
            volume: 0.6,
            muted: false,
            align: 1, // Center
            active_tab: 0,
            dialog_open: false,
            sheet_open: false,
            sheet_side: 1, // Right
            drawer_open: false,
            drawer_side: 2,  // Bottom
            popover_side: 0, // Top
            field_name: String::new(),
            field_username: String::new(),
            field_workspace: String::from("acme-prod"),
            loading: false,
            context_log: None,
            table_page: 0,
            carousel_idx: 0,
            selected_date: Some(Date::new(2025, 6, 12)),
            picker_date: None,
            input_date: Some(Date::new(2025, 6, 1)),
            input_date_text: "June 1, 2025".to_owned(),
            picker_time: glazier::Time::new(10, 30, 0),
            toast_corner: glazier::sonner::Corner::BottomRight,
            chat: vec![
                ChatMsg {
                    id: 0,
                    mine: false,
                    text: "How can I help you today?".to_owned(),
                },
                ChatMsg {
                    id: 1,
                    mine: true,
                    text: "Deploying to prod real quick.".to_owned(),
                },
                ChatMsg {
                    id: 2,
                    mine: false,
                    text: "It's 4:55 PM. On a Friday.".to_owned(),
                },
                ChatMsg {
                    id: 3,
                    mine: true,
                    text: "It's a one-line change.".to_owned(),
                },
                ChatMsg {
                    id: 4,
                    mine: false,
                    text: "It's always a one-line change \u{1f62d}. Alright, let me take a look."
                        .to_owned(),
                },
            ],
            chat_draft: String::new(),
            chat_seq: 5,
            type_size: 1,
            combo_framework: 0,
            combo_timezone: 10, // Europe/London
            command_last: None,
            menubar_last: None,
            nav_last: None,
            otp_code: String::new(),
            attachments: vec![
                ("quarterly-report.pdf".to_owned(), 284_103),
                ("hero-banner.png".to_owned(), 1_572_864),
                ("voice-memo.m4a".to_owned(), 642_000),
            ],
            chart_kind: 0,

            dialog_name: "Pedro Duarte".to_owned(),
            dialog_handle: "@peduarte".to_owned(),
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1480.0, 950.0]),
        ..Default::default()
    };
    let mut dark = false;
    let mut themed = false;
    let mut state = State::default();
    eframe::run_ui_native("glazier gallery", options, move |ui, _frame| {
        if !themed {
            let fonts = glazier::fonts::FontSet {
                regular: include_bytes!("assets/fonts/OpenSans-Regular.ttf").to_vec(),
                semibold: include_bytes!("assets/fonts/OpenSans-Semibold.ttf").to_vec(),
                bold: include_bytes!("assets/fonts/OpenSans-Bold.ttf").to_vec(),
            };
            glazier::fonts::install_with(ui.ctx(), fonts);
            Icon::install(ui.ctx());
            ui.ctx().set_visuals(glazier::shadcn_visuals(dark));
            // Opinionated: prose & headings aren't drag-selectable by default.
            ui.ctx().global_style_mut(glazier::apply_style);
            // Browsers render CSS px at the display's device-pixel-ratio; nudge
            // egui's points-per-px so our 14px base reads at the same density.
            ui.ctx().set_zoom_factor(1.15);
            themed = true;
        }
        // Animate the light↔dark switch: drive a 0→1 value off the `dark` flag
        // and apply a per-frame blend of the two palettes so every token glides
        // instead of snapping. (`animate_bool_with_time` requests the repaints
        // while in flight, then holds the endpoint value.) A short duration plus
        // ease-out keeps it feeling snappy rather than laggy — it moves fast off
        // the mark and decelerates into the new theme.
        let linear = ui
            .ctx()
            .animate_bool_with_time(egui::Id::new("theme_transition"), dark, 0.35);
        let theme_t = ease_out_cubic(linear);
        let blended = Tokens::light().lerp(&Tokens::dark(), theme_t);
        // Pick the egui base by the half-way point to keep shadows/selection sane.
        ui.ctx()
            .set_visuals(glazier::shadcn_visuals_from(blended, theme_t >= 0.5));
        let tokens = Tokens::get(ui);

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(tokens.background))
            .show_inside(ui, |ui| {
                // Paint a subtle top-to-bottom gradient: the page background
                // fades from `background` down into `muted` near the bottom.
                paint_page_gradient(ui, tokens);

                glazier::scroll_area::style_scrollbar(ui, tokens);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Inset the whole page from the window edges.
                        egui::Frame::new()
                            .inner_margin(egui::Margin::symmetric(32, 16))
                            .show(ui, |ui| {
                                // Top-right ghost icon theme toggle.
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::TOP),
                                    |ui| {
                                        if Button::new("")
                                            .variant(Variant::Ghost)
                                            .size(Size::Icon)
                                            .icon_start(Icon::new(lucide::CONTRAST))
                                            .ui(ui)
                                            .clicked()
                                        {
                                            // Flip the flag; the animated blend
                                            // above drives the actual visuals.
                                            dark = !dark;
                                        }
                                    },
                                );

                                hero(ui, tokens);
                                ui.add_space(28.0);
                                showcase_grid(ui, tokens, &mut state);
                                ui.add_space(40.0);
                            });
                    });
            });

        // Drain and render the toast queue over everything else, once per frame.
        glazier::Toaster::new()
            .corner(state.toast_corner)
            .show(ui.ctx());
    })
}

/// The centered hero: badge, headline, subtitle, primary CTA.
fn hero(ui: &mut egui::Ui, _tokens: Tokens) {
    ui.vertical_centered(|ui| {
        ui.add_space(12.0);
        ui.spacing_mut().item_spacing.y = 16.0;

        Badge::new("Introducing the glazier component set")
            .variant(badge::Variant::Secondary)
            .icon_end(Icon::new(lucide::ARROW_RIGHT))
            .ui(ui);

        // shadcn hero: text-5xl/6xl, bold, tight tracking — the `h1` style.
        Typography::h1("The Foundation for your Design System")
            .size(52.0)
            .ui(ui);

        // Cap the prose column at 560px, but never wider than what's available
        // so it narrows (and wraps) instead of overflowing a small window.
        let subtitle_w = 560.0_f32.min(ui.available_width());
        ui.allocate_ui_with_layout(
            Vec2::new(subtitle_w, 0.0),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                Typography::new(
                    "A set of beautifully designed components that you can customize, \
                     extend, and build on. Start here then make it your own.",
                    typography::Variant::Lead,
                )
                .size(18.0)
                .ui(ui);
            },
        );

        ui.add_space(4.0);
        Button::new("Build Your Own")
            .icon_end(Icon::new(lucide::ARROW_RIGHT))
            .ui(ui);
        ui.add_space(20.0);
    });
}

/// Cubic ease-out: fast off the mark, decelerating into the target. Keeps the
/// theme transition feeling responsive instead of a slow linear crawl.
fn ease_out_cubic(t: f32) -> f32 {
    let p = 1.0 - t;
    (p * p).mul_add(-p, 1.0)
}

/// Paint a vertical gradient over the whole viewport: `background` at the top
/// fading into a muted tint near the bottom. Drawn as stacked horizontal bands
/// (egui has no native gradient mesh helper for a simple fill).
///
/// The `muted` token is only ~4% off white, so the fade would be invisible;
/// blend toward `border` instead for a perceptible — but still subtle — degrade.
#[allow(clippy::cast_precision_loss)]
fn paint_page_gradient(ui: &egui::Ui, tokens: Tokens) {
    const BANDS: usize = 64;
    let rect = ui.max_rect();
    let painter = ui.painter();
    let h = rect.height() / BANDS as f32;
    for i in 0..BANDS {
        // Ease the blend so the tint stays clear up top and deepens lower down.
        let t = (i as f32 / (BANDS - 1) as f32).powf(1.6);
        let color = tokens.background.lerp_to_gamma(tokens.border, t);
        let y = h.mul_add(i as f32, rect.top());
        painter.rect_filled(
            egui::Rect::from_min_size(egui::pos2(rect.left(), y), Vec2::new(rect.width(), h + 1.0)),
            0.0,
            color,
        );
    }
}

/// One showcase card, identified so we can render it into whichever column the
/// responsive layout assigns it to.
#[derive(Clone, Copy)]
enum CardId {
    KitchenSink,
    PayoutThreshold,
    Contribution,
    Dividend,
    Claimable,
    AccountAccess,
    Transfer,
    Navigation,
    Controls,
    Disclosure,
    Payments,
    Panes,
    Overlays,
    Forms,
    Data,
    DateTime,
    Conversation,
    Typography,
    CommandPalette,
    Menubar,
    NavMenu,
    InputOtp,
    Attachment,
    Chart,
    Bubble,
    DataTable,
    Sidebar,
}

impl CardId {
    /// A stable small index for this card, used to key cached heights.
    const fn index(self) -> usize {
        self as usize
    }

    /// Render this card into `ui`.
    fn show(self, ui: &mut egui::Ui, tokens: Tokens, state: &mut State) {
        match self {
            Self::KitchenSink => kitchen_sink_card(ui, state),
            Self::PayoutThreshold => payout_threshold_card(ui, tokens, state),
            Self::Contribution => contribution_card(ui, tokens),
            Self::Dividend => dividend_card(ui, tokens, state),
            Self::Claimable => claimable_card(ui, tokens),
            Self::AccountAccess => account_access_card(ui, tokens, state),
            Self::Transfer => transfer_card(ui, tokens, state),
            Self::Navigation => navigation_card(ui, tokens, state),
            Self::Controls => controls_card(ui, tokens, state),
            Self::Disclosure => disclosure_card(ui, tokens, state),
            Self::Payments => payments_card(ui, tokens),
            Self::Panes => panes_card(ui, tokens),
            Self::Overlays => overlays_card(ui, tokens, state),
            Self::Forms => forms_card(ui, tokens, state),
            Self::Data => data_card(ui, tokens, state),
            Self::DateTime => datetime_card(ui, tokens, state),
            Self::Conversation => conversation_card(ui, state),
            Self::Typography => typography_card(ui, tokens, state),
            Self::CommandPalette => command_card(ui, tokens, state),
            Self::Menubar => menubar_card(ui, tokens, state),
            Self::NavMenu => nav_menu_card(ui, tokens, state),
            Self::InputOtp => input_otp_card(ui, tokens, state),
            Self::Attachment => attachment_card(ui, tokens, state),
            Self::Chart => chart_card(ui, tokens, state),
            Self::Bubble => bubble_card(ui, tokens, state),
            Self::DataTable => data_table_card(ui, tokens),
            Self::Sidebar => sidebar_card(ui, tokens, state),
        }
    }
}

/// The masonry grid of component-showcase cards (no separate surface — the
/// cards sit directly on the page gradient).
///
/// Responsive, shadcn-style (`grid-cols-1 md:grid-cols-2 lg:grid-cols-4`): the
/// column count is chosen from the available width, and an ordered card list is
/// dealt round-robin across the columns so each is an independent top-down
/// stack that reflows as the window resizes. With one column the cards simply
/// stack; the cards themselves fill whatever width their column gets.
fn showcase_grid(ui: &mut egui::Ui, tokens: Tokens, state: &mut State) {
    // Breakpoints on the *content* width (roughly shadcn's sm/md/lg rails).
    // Each card wants ~280px before it gets cramped, so step the column count
    // up gradually (1 → 2 → 3 → 4) instead of jumping 2 → 4, which left the
    // cards too narrow and overlapping in the middle band.
    let avail = ui.available_width();
    let columns = if avail < 560.0 {
        1
    } else if avail < 900.0 {
        2
    } else if avail < 1200.0 {
        3
    } else {
        4
    };

    // The ordered card list; the dividend card drops out when dismissed.
    let mut cards = vec![
        CardId::KitchenSink,
        CardId::PayoutThreshold,
        CardId::Contribution,
    ];
    if !state.dividend_dismissed {
        cards.push(CardId::Dividend);
    }
    cards.extend([
        CardId::Claimable,
        CardId::AccountAccess,
        CardId::Transfer,
        CardId::Controls,
        CardId::Disclosure,
        CardId::Payments,
        CardId::Panes,
        CardId::Overlays,
        CardId::Forms,
        CardId::Data,
        CardId::DateTime,
        CardId::Conversation,
        CardId::Typography,
        CardId::CommandPalette,
        CardId::Menubar,
        CardId::NavMenu,
        CardId::InputOtp,
        CardId::Attachment,
        CardId::Chart,
        CardId::Bubble,
        CardId::DataTable,
        CardId::Sidebar,
        CardId::Navigation,
    ]);

    // egui spaces columns horizontally by `item_spacing.x`; reuse that exact
    // value as the vertical gap between stacked cards so row gaps == column gaps
    // (shadcn's single `--gap` token in both axes).
    let gap = ui.spacing().item_spacing.x;

    // Greedy masonry: place each card in the column that is currently shortest,
    // using the height it measured last frame (cards differ a lot in height, so
    // a plain round-robin leaves one column much taller). Heights are cached in
    // egui memory keyed by card index; unseen cards assume a typical height so
    // the very first frame is still reasonable.
    let cache_id = egui::Id::new("gallery-card-heights");
    let heights: Vec<f32> = ui.ctx().memory_mut(|m| {
        m.data
            .get_temp::<std::sync::Arc<Vec<f32>>>(cache_id)
            .map_or_else(Vec::new, |a| (*a).clone())
    });
    let height_of = |card: CardId| -> f32 { heights.get(card.index()).copied().unwrap_or(260.0) };

    // Assign cards to columns by running shortest-column-first.
    let mut col_heights = vec![0.0_f32; columns];
    let mut assigned: Vec<Vec<CardId>> = vec![Vec::new(); columns];
    for card in cards {
        // Index of the currently shortest column.
        let c = col_heights
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map_or(0, |(i, _)| i);
        if !assigned[c].is_empty() {
            col_heights[c] += gap;
        }
        col_heights[c] += height_of(card);
        assigned[c].push(card);
    }

    // Render, capturing each card's actual height for next frame's balance.
    let mut measured = heights.clone();
    ui.columns(columns, |cols| {
        for (c, col_cards) in assigned.into_iter().enumerate() {
            for (i, card) in col_cards.into_iter().enumerate() {
                if i > 0 {
                    cols[c].add_space(gap); // --gap between stacked cards
                }
                let before = cols[c].cursor().top();
                card.show(&mut cols[c], tokens, state);
                let h = cols[c].cursor().top() - before;
                let idx = card.index();
                if measured.len() <= idx {
                    measured.resize(idx + 1, 260.0);
                }
                measured[idx] = h;
            }
        }
    });

    // If measured heights moved since last frame, the balance may now be stale
    // (a card grew/shrank, e.g. an accordion opening) — store them and ask for
    // one more frame so the next layout re-balances. When they've settled this
    // is a no-op, so we don't spin the CPU.
    let changed = measured.len() != heights.len()
        || measured
            .iter()
            .zip(&heights)
            .any(|(a, b)| (a - b).abs() > 0.5);
    ui.ctx().memory_mut(|m| {
        m.data.insert_temp(cache_id, std::sync::Arc::new(measured));
    });
    if changed {
        ui.ctx().request_repaint();
    }
}

// --- Columns of showcase cards ---------------------------------------------

/// A faithful port of shadcn's "kitchen-sink" card: a button row, an input with
/// a trailing search icon, a textarea, then a row of badges + radio group, and a
/// row of checkbox + switch controls.
fn kitchen_sink_card(ui: &mut egui::Ui, state: &mut State) {
    Card::new().show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 16.0;

        // Button row.
        ui.horizontal(|ui| {
            Button::new("Button")
                .size(Size::Small)
                .icon_end(Icon::new(lucide::ARROW_RIGHT))
                .ui(ui);
            Button::new("Secondary")
                .variant(Variant::Secondary)
                .size(Size::Small)
                .ui(ui);
            Button::new("Outline")
                .variant(Variant::Outline)
                .size(Size::Small)
                .ui(ui);
        });

        // Search input group with a trailing lucide search glyph.
        InputGroup::new(&mut state.search)
            .placeholder("Name")
            .icon_end(Icon::new(lucide::SEARCH))
            .ui(ui);

        // Multi-line message.
        Textarea::new(&mut state.message)
            .placeholder("Message")
            .ui(ui);

        // One row: badges on the left, then a right-aligned cluster of
        // radio group + checkbox + switch (matches shadcn's `ml-auto` group).
        ui.horizontal(|ui| {
            Badge::new("Badge").ui(ui);
            Badge::new("Secondary")
                .variant(badge::Variant::Secondary)
                .ui(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                // right_to_left adds the first item rightmost, so order the
                // cluster switch → checkbox → radio to read L→R as radio,
                // checkbox, switch.
                Switch::new(&mut state.compact).ui(ui);
                Checkbox::new(&mut state.email_alerts).ui(ui);
                RadioGroup::new(&mut state.fruit, ["", ""]).ui(ui);
            });
        });

        // Trailing actions: a single outline button + a joined button group.
        ui.horizontal(|ui| {
            if Button::new("Alert Dialog")
                .variant(Variant::Outline)
                .size(Size::Small)
                .ui(ui)
                .clicked()
            {
                state.alert_open = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let picked = ButtonGroup::new(["Button Group"])
                    .icon_segment(Icon::new(lucide::CHEVRON_UP))
                    .menu(
                        DropdownMenu::new()
                            .label("Quick Actions")
                            .item("Mute Conversation")
                            .item("Mark as Read")
                            .item("Block User")
                            .separator()
                            .destructive("Delete Conversation"),
                    )
                    .show(ui)
                    .menu_selected;
                if let Some(i) = picked {
                    state.menu_choice = Some(i);
                }
            });
        });
    });

    // The Alert Dialog button opens a centered modal over a dimmed backdrop.
    // Call every frame so it can animate both in and out.
    AlertDialog::new("Allow accessory to connect?")
        .description(
            "Do you want to allow the USB accessory to connect to this \
             device and your data?",
        )
        .cancel("Don't allow")
        .action("Allow")
        .show(ui.ctx(), &mut state.alert_open);
}

/// Transfer funds card: a header with a dismiss action, an amount field, two
/// account select triggers, a muted summary item, and a confirm button.
fn transfer_card(ui: &mut egui::Ui, tokens: Tokens, state: &mut State) {
    Card::new().show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 6.0;

        // Header: title + description on the left, a dismiss X pinned right.
        // right_to_left lays the button out FIRST (rightmost) with its own
        // rect, then the text column fills the remaining width — so the button
        // gets real interactive bounds instead of being overlapped.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            // size-7 muted icon button.
            Button::new("")
                .variant(Variant::Secondary)
                .size(Size::Icon)
                .icon_start(Icon::new(lucide::X))
                .ui(ui);
            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                Typography::new("Transfer Funds", typography::Variant::Large)
                    .size(15.0)
                    .ui(ui);
                Typography::new(
                    "Move money between your connected accounts.",
                    typography::Variant::Muted,
                )
                .size(13.0)
                .ui(ui);
            });
        });
        ui.add_space(18.0);

        ui.spacing_mut().item_spacing.y = 8.0;

        // Amount field: $ prefix addon. Editing it re-derives the summary below.
        Label::new("Amount to Transfer").ui(ui);
        InputGroup::new(&mut state.transfer_amount)
            .addon_start("$")
            .ui(ui);
        ui.add_space(10.0);

        // Account pickers — real Selects; changing them updates the summary.
        let labels: Vec<String> = ACCOUNTS.iter().map(Account::label).collect();
        Label::new("From Account").ui(ui);
        Select::new(&mut state.from_account, labels.clone()).show(ui);
        ui.add_space(10.0);
        Label::new("To Account").ui(ui);
        Select::new(&mut state.to_account, labels).show(ui);
        ui.add_space(10.0);

        // Live summary, derived from the amount + selected accounts.
        let amount = parse_amount(&state.transfer_amount);
        let same = state.from_account == state.to_account;
        // Cross-bank transfers carry a small fee and arrive later.
        let from = &ACCOUNTS[state.from_account];
        let to = &ACCOUNTS[state.to_account];
        let fee = if same || from.bank == to.bank {
            0.0
        } else {
            2.50
        };
        let arrival = if fee == 0.0 {
            "Today, Apr 14"
        } else {
            "Apr 16–Apr 17"
        };
        let total = amount + fee;

        // Muted summary item with separators between rows.
        Item::new().variant(item::Variant::Muted).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            stat_row(ui, tokens, "Estimated arrival", arrival);
            Separator::new().ui(ui);
            stat_row(ui, tokens, "Transaction fee", &money(fee));
            Separator::new().ui(ui);
            stat_row(ui, tokens, "Total amount", &money(total));
        });
        ui.add_space(12.0);

        // Full-width confirm button (disabled when the transfer is invalid).
        let valid = amount > 0.0 && !same;
        ui.with_layout(
            egui::Layout::top_down_justified(egui::Align::Center),
            |ui| {
                ui.add_enabled_ui(valid, |ui| {
                    Button::new("Confirm Transfer").size(Size::Small).ui(ui);
                });
            },
        );
    });
}

/// Payout threshold card: a settings form — a currency select, a read-only
/// minimum-payout progress gauge with MIN/MAX captions, a notes textarea, and a
/// full-width save action.
fn payout_threshold_card(ui: &mut egui::Ui, _tokens: Tokens, state: &mut State) {
    /// Lower / upper bounds of the payout gauge (`$50 (MIN)` … `$10,000 (MAX)`).
    const MIN_PAYOUT: f64 = 50.0;
    const MAX_PAYOUT: f64 = 10_000.0;

    Card::new().show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 6.0;

        // Header: title + description on the left, a dismiss X pinned right.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            Button::new("")
                .variant(Variant::Secondary)
                .size(Size::Icon)
                .icon_start(Icon::new(lucide::X))
                .ui(ui);
            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                Typography::new("Payout Threshold", typography::Variant::Large)
                    .size(15.0)
                    .ui(ui);
                Typography::new(
                    "Set the minimum balance required before a payout is triggered.",
                    typography::Variant::Muted,
                )
                .size(13.0)
                .ui(ui);
            });
        });
        ui.add_space(18.0);

        // Field 1: preferred currency select.
        let currencies: Vec<&str> = CURRENCIES.iter().map(|c| c.label).collect();
        Label::new("Preferred Currency").ui(ui);
        ui.add_space(2.0);
        Select::new(&mut state.payout_currency, currencies).show(ui);
        ui.add_space(24.0); // gap-6 between fields

        // Field 2: minimum payout gauge. Label on the left, the big tabular
        // figure on the right; a read-only progress track; MIN/MAX captions.
        let sym = CURRENCIES[state.payout_currency].symbol;
        ui.horizontal(|ui| {
            Label::new("Minimum Payout Amount").ui(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                Typography::new(
                    format!("{sym}{:.2}", state.payout_amount),
                    typography::Variant::Large,
                )
                .size(22.0) // text-2xl
                .ui(ui);
            });
        });
        ui.add_space(10.0);
        #[allow(clippy::cast_possible_truncation)]
        let fraction =
            ((state.payout_amount - MIN_PAYOUT) / (MAX_PAYOUT - MIN_PAYOUT)).clamp(0.0, 1.0) as f32;
        Progress::new(fraction).ui(ui);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            Typography::new(format!("{sym}50 (MIN)"), typography::Variant::Muted)
                .size(13.0)
                .ui(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                Typography::new(format!("{sym}10,000 (MAX)"), typography::Variant::Muted)
                    .size(13.0)
                    .ui(ui);
            });
        });
        ui.add_space(24.0);

        // Field 3: free-form notes.
        Label::new("Notes").ui(ui);
        ui.add_space(2.0);
        Textarea::new(&mut state.payout_notes)
            .placeholder("Add any notes for this payout configuration...")
            .rows(4)
            .ui(ui);
        ui.add_space(16.0);

        // Full-width primary save button.
        ui.with_layout(
            egui::Layout::top_down_justified(egui::Align::Center),
            |ui| {
                Button::new("Save Threshold").size(Size::Small).ui(ui);
            },
        );
    });
}

/// A selectable currency in the payout card, with its display symbol.
struct Currency {
    label: &'static str,
    symbol: &'static str,
}

/// The payout card's currency options.
const CURRENCIES: [Currency; 4] = [
    Currency {
        label: "USD \u{2014} United States Dollar",
        symbol: "$",
    },
    Currency {
        label: "EUR \u{2014} Euro",
        symbol: "\u{20ac}",
    },
    Currency {
        label: "GBP \u{2014} British Pound",
        symbol: "\u{a3}",
    },
    Currency {
        label: "JPY \u{2014} Japanese Yen",
        symbol: "\u{a5}",
    },
];

/// A connected account shown in the transfer card's pickers.
struct Account {
    name: &'static str,
    last4: &'static str,
    balance: f64,
    bank: &'static str,
}

impl Account {
    /// The trigger/option label, e.g. `Main Checking (··8402) — $12,450.00`.
    fn label(&self) -> String {
        format!("{} (··{}) — {}", self.name, self.last4, money(self.balance))
    }
}

/// The demo's connected accounts.
const ACCOUNTS: [Account; 3] = [
    Account {
        name: "Main Checking",
        last4: "8402",
        balance: 12_450.00,
        bank: "acme",
    },
    Account {
        name: "High Yield Savings",
        last4: "1192",
        balance: 42_100.00,
        bank: "acme",
    },
    Account {
        name: "Brokerage Cash",
        last4: "5530",
        balance: 8_900.00,
        bank: "vimco",
    },
];

/// Parse a user-typed amount like `1,200.00` into a number (commas ignored).
fn parse_amount(text: &str) -> f64 {
    text.replace([',', '$', ' '], "").parse().unwrap_or(0.0)
}

/// Format a number as `$1,200.00` with thousands separators.
#[allow(clippy::cast_possible_truncation)]
fn money(value: f64) -> String {
    let cents = (value * 100.0).round() as i64;
    let dollars = cents / 100;
    let frac = (cents % 100).abs();
    // Group the integer part into comma-separated thousands.
    let digits = dollars.abs().to_string();
    let mut grouped = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let sign = if dollars < 0 { "-" } else { "" };
    format!("{sign}${grouped}.{frac:02}")
}

/// Contribution history card: a `chart-2` bar chart with month labels, two
/// muted stat items, and a full-width footer action.
fn contribution_card(ui: &mut egui::Ui, tokens: Tokens) {
    const MONTHS: [&str; 6] = ["Dec", "Jan", "Feb", "Mar", "Apr", "May"];
    const HEIGHTS: [f32; 6] = [0.5714, 0.7857, 0.6429, 0.9286, 0.5357, 1.0];

    Card::new()
        .title("Contribution History")
        .description("Last 6 months of activity")
        .show(ui, |ui| {
            bar_chart(ui, tokens, &HEIGHTS, &MONTHS);
            ui.add_space(12.0);

            // Two muted stat items, side by side (shadcn `xl:grid-cols-2`).
            ui.columns(2, |cols| {
                stat_item(&mut cols[0], tokens, "Upcoming", "May 2024", "Scheduled");
                stat_item(
                    &mut cols[1],
                    tokens,
                    "Savings Plan",
                    "Accelerated",
                    "Recurring",
                );
            });
            ui.add_space(12.0);

            // Full-width primary footer button.
            ui.with_layout(
                egui::Layout::top_down_justified(egui::Align::Center),
                |ui| {
                    Button::new("View Full Report").size(Size::Small).ui(ui);
                },
            );
        });
}

/// shadcn's `--chart-2` token (teal in light, green in dark), converted from
/// OKLCH to sRGB.
const fn chart_2(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0x00, 0xbc, 0x7d)
    } else {
        Color32::from_rgb(0x00, 0x96, 0x89)
    }
}

/// One muted stat item: an uppercase caption, a semibold value, and a muted
/// sub-line (shadcn `<Item variant="muted">`).
fn stat_item(ui: &mut egui::Ui, _tokens: Tokens, caption: &str, value: &str, sub: &str) {
    Item::new().variant(item::Variant::Muted).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 4.0; // gap-1
                                               // Uppercase muted caption (`text-xs`).
        Typography::new(caption.to_uppercase(), typography::Variant::Muted)
            .size(11.0)
            .ui(ui);
        // Semibold value (`text-base font-semibold`).
        Typography::new(value, typography::Variant::Large)
            .size(16.0)
            .ui(ui);
        // Muted sub-line (`text-sm`).
        Typography::new(sub, typography::Variant::Muted)
            .size(13.0)
            .ui(ui);
    });
}

/// Claimable balance card: shadcn card-inside-card (Item) layout.
fn claimable_card(ui: &mut egui::Ui, tokens: Tokens) {
    Card::new()
        .description("Claimable Balance")
        .footer(
            "Once your bank is connected, balances over $10.00 are automatically \
             eligible for monthly distribution on the 15th of each month.",
        )
        .show(ui, |ui| {
            // Big balance figure (text-4xl, medium weight — not bold).
            Typography::new("$1,211.29", typography::Variant::Large)
                .size(36.0)
                .ui(ui);
            ui.add_space(8.0);
            // Outline badge with a yellow status dot. Wrap in a left-to-right
            // layout so it stays `w-fit` instead of stretching to the column.
            ui.horizontal(|ui| {
                Badge::new("Pending Setup")
                    .variant(badge::Variant::Outline)
                    .dot(Color32::from_rgb(0xf0, 0xb1, 0x00)) // tailwind yellow-500
                    .ui(ui);
            });
            ui.add_space(16.0);
            // Inner muted Item grouping the breakdown rows (gap-3).
            Item::new().variant(item::Variant::Muted).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                stat_row(ui, tokens, "Net Royalties", "$1,248.75");
                stat_row(ui, tokens, "Processing Fee", "-$37.46");
                Separator::new().ui(ui);
                stat_row(ui, tokens, "Total Ready to Claim", "$1,211.29 USD");
            });
        });
}

/// One dividend holding shown in the Q2 card: name, share count, and four
/// quarterly bar heights (0..1) for its mini sparkline.
struct Holding {
    name: &'static str,
    shares: &'static str,
    bars: [f32; 4],
}

/// The Q2 dividend holdings (bar heights ported from the shadcn markup).
const HOLDINGS: [Holding; 4] = [
    Holding {
        name: "Vanguard",
        shares: "450 Shares",
        bars: [0.583, 0.644, 0.598, 1.0],
    },
    Holding {
        name: "S&P 500 VOO",
        shares: "112 Shares",
        bars: [0.563, 0.656, 1.0, 0.681],
    },
    Holding {
        name: "Apple AAPL",
        shares: "85 Shares",
        bars: [0.50, 0.583, 1.0, 0.75],
    },
    Holding {
        name: "Realty Income",
        shares: "320 Shares",
        bars: [0.667, 0.722, 0.778, 1.0],
    },
];

/// Q2 Dividend Income card: a dismissable header over a list of muted holding
/// items, each pairing a name + share count with a four-bar `chart-2`
/// sparkline of quarterly payouts.
fn dividend_card(ui: &mut egui::Ui, _tokens: Tokens, state: &mut State) {
    Card::new().show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 6.0;

        // Header: title + description on the left, a dismiss X pinned right.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            if Button::new("")
                .variant(Variant::Secondary)
                .size(Size::Icon)
                .icon_start(Icon::new(lucide::X))
                .ui(ui)
                .clicked()
            {
                state.dividend_dismissed = true;
            }
            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                Typography::new("Q2 Dividend Income", typography::Variant::Large)
                    .size(15.0)
                    .ui(ui);
                Typography::new(
                    "Quarterly dividend payouts across your portfolio holdings.",
                    typography::Variant::Muted,
                )
                .size(13.0)
                .ui(ui);
            });
        });
        ui.add_space(18.0);

        // Holding list (gap-4 between muted items).
        ui.spacing_mut().item_spacing.y = 16.0;
        for holding in &HOLDINGS {
            Item::new().variant(item::Variant::Muted).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        Typography::new(holding.name, typography::Variant::Small)
                            .size(13.0)
                            .ui(ui);
                        Typography::new(holding.shares, typography::Variant::Muted)
                            .size(13.0)
                            .ui(ui);
                    });
                    // Sparkline pinned to the right edge (w-24 h-8).
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        mini_bars(ui, &holding.bars);
                    });
                });
            });
        }
    });
}

/// A compact four-bar sparkline (shadcn's `w-24 h-8` dividend chart): fixed
/// 96×32 box, `chart-2` bars with rounded tops and a `min-h-1` floor.
#[allow(clippy::cast_precision_loss)]
fn mini_bars(ui: &mut egui::Ui, values: &[f32]) {
    const W: f32 = 96.0; // w-24
    const H: f32 = 32.0; // h-8
    const GAP: f32 = 4.0; // gap-1
    const MIN_BAR: f32 = 4.0; // min-h-1

    let color = chart_2(ui.visuals().dark_mode);
    let (rect, _) = ui.allocate_at_least(Vec2::new(W, H), egui::Sense::hover());
    let n = values.len();
    let bar_w = GAP.mul_add(-(n as f32 - 1.0), rect.width()) / n as f32;
    // `rounded-t-sm`: round the top corners only.
    let r = 2;
    let top_round = egui::CornerRadius {
        nw: r,
        ne: r,
        sw: 0,
        se: 0,
    };
    let baseline = rect.bottom();
    for (i, v) in values.iter().enumerate() {
        let x = (i as f32).mul_add(bar_w + GAP, rect.left());
        let h = (H * v.clamp(0.0, 1.0)).max(MIN_BAR);
        let bar = egui::Rect::from_min_size(egui::pos2(x, baseline - h), Vec2::new(bar_w, h));
        ui.painter().rect_filled(bar, top_round, color);
    }
}

/// An interactive `<a>`-style [`Item`]: a muted inset row that darkens from
/// `bg-muted/50` to full `bg-muted` on hover (shadcn's `[a]:hover:bg-muted`),
/// with a pointing-hand cursor. Mirrors `Item`'s chrome (radius-2xl, 14px pad).
/// Returns the [`Response`] so callers can act on `.clicked()`.
fn hover_item(
    ui: &mut egui::Ui,
    id_salt: &str,
    content: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    Item::new().interactive(id_salt).show(ui, content)
}

/// An interactive inline text link (shadcn's `<a>`): muted by default, recolours
/// to `foreground` on hover, shows a pointing-hand cursor, and is clickable.
/// Returns the [`Response`] so callers can act on `.clicked()`.
fn link(ui: &mut egui::Ui, tokens: Tokens, text: RichText) -> egui::Response {
    // Lay the text out first so we can size the clickable rect, then choose the
    // colour from the hover state before painting (egui paints immediately).
    let widget_text: egui::WidgetText = text.into();
    let galley = widget_text.into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        ui.available_width(),
        egui::TextStyle::Body,
    );
    let (rect, response) = ui.allocate_exact_size(galley.size(), egui::Sense::click());
    let hover_t =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.15);
    let color = tokens
        .muted_foreground
        .lerp_to_gamma(tokens.foreground, hover_t);
    ui.painter().galley(rect.min, galley, color);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Account Access card: a credentials form, a full-width primary "Update
/// Security" button, and a destructive "Danger Zone" link-item footer.
fn account_access_card(ui: &mut egui::Ui, tokens: Tokens, state: &mut State) {
    Card::new()
        .title("Account Access")
        .description("Update your credentials or re-authenticate.")
        .show(ui, |ui| {
            ui.add_space(12.0);
            ui.spacing_mut().item_spacing.y = 12.0; // gap-3 within a field

            // Field 1: email address.
            Label::new("Email Address").ui(ui);
            Input::new(&mut state.access_email)
                .placeholder("artist@studio.inc")
                .ui(ui);
            ui.add_space(12.0); // gap-6 between fields

            // Field 2: current password, with a right-aligned "FORGOT?" link.
            ui.horizontal(|ui| {
                Label::new("Current Password").ui(ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // text-xs uppercase tracking-wider, hover:text-foreground
                    link(ui, tokens, RichText::new("FORGOT?").size(11.0));
                });
            });
            Input::new(&mut state.access_password)
                .password(true)
                .placeholder("Enter current password")
                .ui(ui);
            ui.add_space(20.0); // card footer gap

            // Footer: full-width primary action with a leading lock glyph.
            ui.with_layout(
                egui::Layout::top_down_justified(egui::Align::Center),
                |ui| {
                    Button::new("Update Security")
                        .size(Size::Small)
                        .icon_start(Icon::new(lucide::LOCK))
                        .ui(ui);
                },
            );
            ui.add_space(16.0); // gap-4

            // Destructive "Danger Zone" link-item: alert icon, title +
            // description, trailing chevron. Darkens on hover ([a]:hover:bg-muted).
            hover_item(ui, "danger-zone", |ui| {
                ui.horizontal(|ui| {
                    Icon::new(lucide::CIRCLE_ALERT)
                        .color(tokens.destructive)
                        .ui(ui);
                    ui.add_space(2.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        Typography::new("Danger Zone", typography::Variant::Small)
                            .size(13.0)
                            .ui(ui);
                        Typography::new(
                            "Archive account and remove catalog",
                            typography::Variant::Muted,
                        )
                        .size(13.0)
                        .ui(ui);
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        Icon::new(lucide::CHEVRON_RIGHT)
                            .color(tokens.muted_foreground)
                            .ui(ui);
                    });
                });
            });
        });
}

/// Navigation cards: a 2×2 [`Grid`] of four standalone cards, each holding one
/// [`SidebarMenu`] group (Overview / Planning / Support / Account) — a labelled
/// stack of icon+label nav rows with one active item. Mirrors the markup's four
/// separate `card` slots laid out `grid-cols-2 gap-4`.
///
/// These cards are `py-0`: the menu's own `p-2` is the *only* padding, so we use
/// a bare card-styled frame (zero inner margin) instead of [`Card`], whose 20px
/// margin would double up with the group padding.
fn navigation_card(ui: &mut egui::Ui, tokens: Tokens, state: &mut State) {
    /// A card surface with no inner padding (`rounded-3xl py-0`).
    fn nav_frame(ui: &mut egui::Ui, tokens: Tokens, content: impl FnOnce(&mut egui::Ui)) {
        egui::Frame::new()
            .fill(tokens.card)
            .stroke(egui::Stroke::new(1.0, tokens.border))
            .corner_radius(tokens.radius_3xl())
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                content(ui);
            });
    }

    Grid::new(2).gap(16.0).show(ui, 4, |idx, ui| match idx {
        0 => nav_frame(ui, tokens, |ui| {
            SidebarMenu::new("Overview")
                .item(lucide::ANALYTICS, "Analytics")
                .item(lucide::TRANSACTIONS, "Transactions")
                .item(lucide::INVESTMENTS, "Investments")
                .item(lucide::ACCOUNTS_ICON, "Accounts")
                .item(lucide::SPENDING, "Spending")
                .show(ui, &mut state.nav_overview);
        }),
        1 => nav_frame(ui, tokens, |ui| {
            SidebarMenu::new("Planning")
                .item(lucide::DOCUMENTS, "Documents")
                .item(lucide::BUDGET, "Budget")
                .item(lucide::REPORTS, "Reports")
                .item(lucide::GOALS, "Goals")
                .item(lucide::CALENDAR, "Calendar")
                .show(ui, &mut state.nav_planning);
        }),
        2 => nav_frame(ui, tokens, |ui| {
            SidebarMenu::new("Support")
                .item(lucide::HELP, "Help Center")
                .item(lucide::DOCS, "Docs")
                .item(lucide::CONTACT, "Contact Us")
                .item(lucide::STATUS, "Status")
                .item(lucide::COMMUNITY, "Community")
                .show(ui, &mut state.nav_support);
        }),
        _ => nav_frame(ui, tokens, |ui| {
            SidebarMenu::new("Account")
                .item(lucide::PROFILE, "Profile")
                .item(lucide::BILLING, "Billing")
                .item(lucide::BELL, "Notifications")
                .item(lucide::SHIELD, "Security")
                .item(lucide::APPEARANCE, "Appearance")
                .show(ui, &mut state.nav_account);
        }),
    });
}

/// A card showcasing the Tier-1 value controls: a [`Slider`] bound to a volume
/// value (with a [`Toggle`] mute button), and a single-select [`ToggleGroup`]
/// for text alignment.
fn controls_card(ui: &mut egui::Ui, _tokens: Tokens, state: &mut State) {
    Card::new().show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 16.0;

        Typography::new("Playback", typography::Variant::Large)
            .size(15.0)
            .ui(ui);

        // Volume slider + a mute toggle button. Pin the row to the Small
        // toggle's height (32px) and centre vertically so the `%` readout and
        // the button share one centreline.
        ui.horizontal(|ui| {
            ui.set_min_height(32.0);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let frac = if state.muted { 0.0 } else { state.volume };
                Typography::new(format!("{:.0}%", frac * 100.0), typography::Variant::Muted)
                    .size(12.0)
                    .ui(ui);
                Toggle::new(&mut state.muted, "Mute")
                    .variant(glazier::toggle::Variant::Outline)
                    .size(glazier::toggle::Size::Small)
                    .ui(ui);
            });
        });
        // While muted the slider shows (and edits) a parked zero.
        let mut zero = 0.0;
        let value = if state.muted {
            &mut zero
        } else {
            &mut state.volume
        };
        Slider::new(value, 0.0..=1.0).ui(ui);

        Separator::new().ui(ui);

        // Text-alignment segmented control.
        Typography::new("Alignment", typography::Variant::Large)
            .size(15.0)
            .ui(ui);
        ToggleGroup::single(&mut state.align, ["Left", "Center", "Right"]).ui(ui);
    });
}

/// A card showcasing the Tier-2 disclosure components: two [`Alert`] callouts
/// (default + destructive), a single [`Collapsible`], and a single-open
/// [`Accordion`].
fn disclosure_card(ui: &mut egui::Ui, _tokens: Tokens, state: &mut State) {
    Card::new().show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 12.0;

        // Pill-style tabs over a tiny switchable panel.
        Tabs::new(&mut state.active_tab, ["Overview", "Activity", "Settings"]).show(ui, |ui, i| {
            let body = match i {
                0 => "Your account at a glance.",
                1 => "Recent sign-ins and changes.",
                _ => "Manage preferences and keys.",
            };
            Typography::new(body, typography::Variant::Muted)
                .size(13.0)
                .ui(ui);
        });

        Separator::new().ui(ui);

        Alert::new("Heads up!")
            .description("You can add components to your app using the CLI.")
            .icon(Icon::new(lucide::INFO))
            .ui(ui);

        Alert::new("Session expired")
            .description("Please log in again to continue.")
            .icon(Icon::new(lucide::CIRCLE_ALERT))
            .variant(alert::Variant::Destructive)
            .ui(ui);

        Separator::new().ui(ui);

        Collapsible::new("recovery-keys")
            .title("Show recovery keys")
            .show(ui, |ui| {
                Typography::new("9f3a-22b1-77de\n4c0e-8a91-2bd5", typography::Variant::Muted)
                    .size(13.0)
                    .ui(ui);
            });

        Separator::new().ui(ui);

        Accordion::new("faq")
            .section("Is it accessible?", |ui| {
                ui.label("Yes. It follows the WAI-ARIA design pattern.");
            })
            .section("Is it styled?", |ui| {
                ui.label("Yes, it inherits the active theme's tokens.");
            })
            .section("Is it animated?", |ui| {
                ui.label("Yes, the reveal uses a height transition.");
            })
            .show(ui);
    });
}

/// A faithful port of shadcn's "Payments" card: a breadcrumb header
/// (Home › ⋯ › Payments) above a group of three muted item-rows, each an
/// icon + title + description with a trailing chevron.
fn payments_card(ui: &mut egui::Ui, tokens: Tokens) {
    Card::new().show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 16.0;

        // Header: breadcrumb trail with a collapsed options ellipsis.
        Breadcrumb::new()
            .link("Home")
            .ellipsis()
            .page("Payments")
            .show(ui);

        // Item group: three muted action rows (gap-4).
        ui.spacing_mut().item_spacing.y = 16.0;
        payment_item(
            ui,
            tokens,
            lucide::SETTINGS,
            "limit",
            "Change transfer limit",
            "Adjust how much you can send from your balance.",
        );
        payment_item(
            ui,
            tokens,
            lucide::CALENDAR,
            "scheduled",
            "Scheduled transfers",
            "Set up a transfer to send at a later date.",
        );
        payment_item(
            ui,
            tokens,
            lucide::REPEAT,
            "recurring",
            "Recurring card payments",
            "Manage your repeated card transactions.",
        );
    });
}

/// One `data-variant="muted"` item row from the Payments card: a leading icon,
/// a title + muted description column, and a trailing chevron. The whole row is
/// a `bg-muted/50` surface that fills to full `muted` on hover (shadcn's
/// `[a]:hover:bg-muted`), with a pointing-hand cursor.
fn payment_item(
    ui: &mut egui::Ui,
    tokens: Tokens,
    icon: &'static str,
    id_salt: &str,
    title: &str,
    description: &str,
) {
    Item::new().interactive(id_salt).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0; // gap-3.5
                                                    // Leading media icon, top-aligned with the title.
            Icon::new(icon).color(tokens.foreground).size(16.0).ui(ui);
            // Title + description column (gap-1).
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                Typography::new(title, typography::Variant::Small)
                    .size(13.0)
                    .ui(ui);
                Typography::new(description, typography::Variant::Muted)
                    .size(13.0)
                    .ui(ui);
            });
            // Trailing chevron, right-aligned and vertically centred.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                Icon::new(lucide::CHEVRON_RIGHT)
                    .color(tokens.muted_foreground)
                    .size(16.0)
                    .ui(ui);
            });
        });
    });
}

/// Showcase for [`ScrollArea`] and [`Resizable`]: shadcn's bordered "Tags"
/// scroll list beside a draggable two-pane split.
fn panes_card(ui: &mut egui::Ui, tokens: Tokens) {
    Card::new()
        .title("Panes")
        .description("Scrollable viewport and a draggable split.")
        .show(ui, |ui| {
            ui.add_space(8.0);

            // shadcn's canonical ScrollArea "Tags" demo: a bordered box that
            // grows to fill the row. The "Tags" header is pinned at the top
            // (sticky) while the separator-delimited version list scrolls
            // beneath it. (shadcn defaults to no scroll-snap, so neither do we.)
            egui::Frame::new()
                .stroke(egui::Stroke::new(1.0, tokens.border))
                .corner_radius(tokens.radius_md())
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    // Sticky header.
                    Typography::new("Tags", typography::Variant::Small)
                        .size(13.0)
                        .ui(ui);
                    ui.add_space(8.0);
                    // Scrolling list below the pinned header.
                    ScrollArea::new().max_height(260.0).show(ui, |ui| {
                        for i in (1..=50).rev() {
                            Typography::new(format!("v1.2.0-beta.{i}"), typography::Variant::P)
                                .size(13.0)
                                .ui(ui);
                            ui.add_space(6.0);
                            Separator::new().ui(ui);
                            ui.add_space(6.0);
                        }
                    });
                });

            ui.add_space(16.0);
            Separator::new().ui(ui);
            ui.add_space(16.0);

            // A draggable two-pane split with a grip handle.
            Resizable::new("panes-demo")
                .size(84.0)
                .default_fraction(0.4)
                .handle_grip(true)
                .show(
                    ui,
                    |ui| {
                        ui.centered_and_justified(|ui| {
                            Typography::new("Sidebar", typography::Variant::Muted).ui(ui);
                        });
                    },
                    |ui| {
                        ui.centered_and_justified(|ui| {
                            Typography::new("Content", typography::Variant::Muted).ui(ui);
                        });
                    },
                );
        });
}

/// Showcase for the overlay primitives: a [`Tooltip`] on a hover target, a
/// click-anchored [`Popover`] hosting a little form, and a modal [`Dialog`] with
/// an edit form.
fn overlays_card(ui: &mut egui::Ui, tokens: Tokens, state: &mut State) {
    Card::new()
        .title("Overlays")
        .description("Tooltips, popovers, dialogs, sheets, drawers, and toasts.")
        .show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;

                // Tooltip: hover the outline button to reveal the dark bubble.
                let hover = Button::new("Hover")
                    .variant(Variant::Outline)
                    .size(Size::Small)
                    .ui(ui);
                Tooltip::new("Add to library").show(ui, &hover);

                // Dialog: opens a centered modal with an edit form.
                if Button::new("Dialog")
                    .variant(Variant::Outline)
                    .size(Size::Small)
                    .ui(ui)
                    .clicked()
                {
                    state.dialog_open = true;
                }
            });

            // Popover on its own row: the trigger plus a cluster selector for
            // which side it opens towards (kept apart so the segments don't
            // overflow the row above).
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;

                // Popover: click to open a small panel with content, opening
                // towards the picked side.
                let popover_align = match state.popover_side {
                    1 => egui::RectAlign::RIGHT,
                    2 => egui::RectAlign::BOTTOM,
                    3 => egui::RectAlign::LEFT,
                    _ => egui::RectAlign::TOP,
                };
                let trigger = Button::new("Popover")
                    .variant(Variant::Secondary)
                    .size(Size::Small)
                    .ui(ui);
                Popover::new()
                    .width(240.0)
                    .align(popover_align)
                    .show(ui, &trigger, |ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        Typography::new("Dimensions", typography::Variant::Small)
                            .size(14.0)
                            .ui(ui);
                        Typography::new(
                            "Set the dimensions for the layer.",
                            typography::Variant::Muted,
                        )
                        .size(13.0)
                        .ui(ui);
                        Separator::new().ui(ui);
                        stat_row(ui, tokens, "Width", "100%");
                        stat_row(ui, tokens, "Max width", "300px");
                        stat_row(ui, tokens, "Height", "25px");
                    });

                ToggleGroup::single(&mut state.popover_side, ["Top", "Right", "Bottom", "Left"])
                    .ui(ui);
            });

            // Sheet on its own row: the trigger plus a cluster selector for
            // which edge it enters from.
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;

                if Button::new("Sheet")
                    .variant(Variant::Outline)
                    .size(Size::Small)
                    .ui(ui)
                    .clicked()
                {
                    state.sheet_open = true;
                }

                ToggleGroup::single(&mut state.sheet_side, ["Top", "Right", "Bottom", "Left"])
                    .ui(ui);
            });

            // Drawer on its own row: the trigger plus a cluster selector for
            // which edge it enters from (kept apart so the segments don't
            // overflow the row above).
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;

                if Button::new("Drawer")
                    .variant(Variant::Outline)
                    .size(Size::Small)
                    .ui(ui)
                    .clicked()
                {
                    state.drawer_open = true;
                }

                ToggleGroup::single(&mut state.drawer_side, ["Top", "Right", "Bottom", "Left"])
                    .ui(ui);
            });

            // Sonner toasts: fire-and-forget notifications, drained app-wide.
            ui.add_space(16.0);
            Separator::new().ui(ui);
            ui.add_space(16.0);
            Typography::new("Toasts", typography::Variant::Small)
                .size(13.0)
                .ui(ui);
            ui.add_space(8.0);
            toast_triggers(ui, tokens, state);
        });

    // The modal itself paints on the context above the page; call every frame
    // so it animates both in and out.
    let State {
        dialog_open,
        dialog_name,
        dialog_handle,
        ..
    } = state;
    let mut saved = false;
    Dialog::new("Edit profile")
        .description("Make changes to your profile here. Click save when you're done.")
        .width(420.0)
        .show(ui.ctx(), dialog_open, |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;

            Label::new("Name").ui(ui);
            Input::new(dialog_name).ui(ui);

            Label::new("Username").ui(ui);
            Input::new(dialog_handle).ui(ui);

            ui.add_space(4.0);
            // Footer: a right-aligned primary save action.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                saved = Button::new("Save changes")
                    .size(Size::Small)
                    .ui(ui)
                    .clicked();
            });
        });
    if saved {
        *dialog_open = false;
    }

    // Sheet: an edge-anchored panel sliding in from the picked edge.
    let sheet_side = match state.sheet_side {
        0 => Side::Top,
        1 => Side::Right,
        3 => Side::Left,
        _ => Side::Bottom,
    };
    let State {
        sheet_open,
        dialog_name: sheet_name,
        ..
    } = state;
    Sheet::new("Edit profile")
        .side(sheet_side)
        .description("Make changes to your profile here.")
        .show(ui.ctx(), sheet_open, |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            Label::new("Name").ui(ui);
            Input::new(sheet_name).ui(ui);
            Typography::new(
                "Close from the ×, the backdrop, or Escape.",
                typography::Variant::Muted,
            )
            .size(13.0)
            .ui(ui);
        });

    // Drawer: an edge-anchored panel with a grip, entering from the picked side.
    let drawer_side = match state.drawer_side {
        0 => Side::Top,
        1 => Side::Right,
        3 => Side::Left,
        _ => Side::Bottom,
    };
    let mut submitted = false;
    Drawer::new("Move goal")
        .side(drawer_side)
        .description("Set your daily activity goal.")
        .show(ui.ctx(), &mut state.drawer_open, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                Typography::new("350 calories / day", typography::Variant::Large)
                    .size(22.0)
                    .ui(ui);
                ui.add_space(8.0);
                submitted = Button::new("Submit").size(Size::Small).ui(ui).clicked();
            });
        });
    if submitted {
        state.drawer_open = false;
    }
}

/// Showcase for [`Field`], [`Spinner`], [`Empty`], and [`ContextMenu`]: a small
/// sign-in form with validation, a loading toggle, an empty-state panel, and a
/// right-click menu.
fn forms_card(ui: &mut egui::Ui, tokens: Tokens, state: &mut State) {
    Card::new()
        .title("Forms")
        .description("Fields, spinner, empty state, and a context menu.")
        .show(ui, |ui| {
            ui.add_space(8.0);
            ui.spacing_mut().item_spacing.y = 14.0;

            // Two labelled fields; the username shows an inline error when short.
            Field::new("Display name")
                .description("This is your public name.")
                .show(ui, |ui| Input::new(&mut state.field_name).ui(ui));

            let name_len = state.field_username.chars().count();
            let invalid = name_len > 0 && name_len < 3;
            let mut username = Field::new("Username");
            if invalid {
                username = username.error("Must be at least 3 characters.");
            } else {
                username = username.description("Used in your profile URL.");
            }
            username.show(ui, |ui| {
                Input::new(&mut state.field_username)
                    .placeholder("username")
                    .invalid(invalid)
                    .ui(ui)
            });

            // A disabled field can't be edited and reads muted.
            Field::new("Workspace")
                .description("Managed by your administrator.")
                .show(ui, |ui| {
                    Input::new(&mut state.field_workspace).disabled(true).ui(ui)
                });

            Separator::new().ui(ui);

            // A loading toggle: the Spinner animates while `loading` is on.
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if Button::new(if state.loading { "Stop" } else { "Load" })
                    .variant(Variant::Outline)
                    .size(Size::Small)
                    .ui(ui)
                    .clicked()
                {
                    state.loading = !state.loading;
                }
                if state.loading {
                    Spinner::new()
                        .size(18.0)
                        .style(move |s: &mut glazier::spinner::SpinnerStyle| {
                            s.color = tokens.muted_foreground;
                        })
                        .ui(ui);
                    Typography::new("Loading…", typography::Variant::Muted)
                        .size(13.0)
                        .ui(ui);
                }
            });

            Separator::new().ui(ui);

            // An empty-state panel; the title row accepts a right-click menu.
            let panel = egui::Frame::new()
                .stroke(egui::Stroke::new(1.0, tokens.border))
                .corner_radius(tokens.radius_md())
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    Empty::new("No messages")
                        .icon(Icon::new(lucide::INBOX))
                        .description("Right-click for actions.")
                        .show(ui, |ui| {
                            if Button::new("Compose").size(Size::Small).ui(ui).clicked() {
                                state.context_log = Some("Compose".to_owned());
                            }
                        });
                });
            let menu = DropdownMenu::new()
                .label("Inbox")
                .item("Refresh")
                .item("Mark all read")
                .separator()
                .destructive("Empty trash");
            if let Some(i) = ContextMenu::new(menu).show(ui, &panel.response) {
                let action = ["Refresh", "Mark all read", "Empty trash"]
                    .get(i)
                    .copied()
                    .unwrap_or("?");
                state.context_log = Some(action.to_owned());
            }

            if let Some(last) = &state.context_log {
                Typography::new(format!("Last action: {last}"), typography::Variant::Muted)
                    .size(13.0)
                    .ui(ui);
            }
        });
}

/// Showcase for [`Table`], [`Pagination`], and [`Carousel`]: a paginated
/// invoices table above a small image-free slide carousel.
fn data_card(ui: &mut egui::Ui, tokens: Tokens, state: &mut State) {
    // Mock invoice rows, paged five at a time.
    const ROWS: &[(&str, &str, &str)] = &[
        ("INV001", "Paid", "$250.00"),
        ("INV002", "Pending", "$150.00"),
        ("INV003", "Unpaid", "$350.00"),
        ("INV004", "Paid", "$450.00"),
        ("INV005", "Paid", "$550.00"),
        ("INV006", "Pending", "$200.00"),
        ("INV007", "Unpaid", "$300.00"),
        ("INV008", "Paid", "$100.00"),
        ("INV009", "Pending", "$175.00"),
        ("INV010", "Paid", "$325.00"),
        ("INV011", "Unpaid", "$225.00"),
        ("INV012", "Paid", "$275.00"),
    ];
    const PER_PAGE: usize = 5;
    let pages = ROWS.len().div_ceil(PER_PAGE);

    Card::new()
        .title("Data")
        .description("Table with pagination, and a carousel.")
        .show(ui, |ui| {
            ui.add_space(8.0);

            let page = state.table_page.min(pages - 1);
            let start = page * PER_PAGE;
            let mut table = Table::new()
                .min_rows(PER_PAGE)
                .column("Invoice", Sizing::Auto, egui::Align::LEFT)
                .column("Status", Sizing::Remainder, egui::Align::LEFT)
                .column("Amount", Sizing::Auto, egui::Align::RIGHT);
            for (inv, status, amount) in ROWS.iter().skip(start).take(PER_PAGE) {
                table = table.row([*inv, *status, *amount]);
            }
            table.ui(ui);

            ui.add_space(12.0);
            Pagination::new(pages).show(ui, &mut state.table_page);

            ui.add_space(16.0);
            Separator::new().ui(ui);
            ui.add_space(16.0);

            // A small carousel of coloured slides.
            Carousel::new(5)
                .height(120.0)
                .show(ui, &mut state.carousel_idx, |ui, i| {
                    let frame = egui::Frame::new()
                        .corner_radius(tokens.radius_md())
                        .fill(tokens.muted);
                    frame.show(ui, |ui| {
                        ui.set_min_size(ui.available_size());
                        ui.centered_and_justified(|ui| {
                            Typography::new(format!("Slide {}", i + 1), typography::Variant::Large)
                                .size(26.0)
                                .color(tokens.muted_foreground)
                                .ui(ui);
                        });
                    });
                });
        });
}

/// Showcase for [`Calendar`] and the [`DatePicker`](glazier::DatePicker): an
/// inline month picker plus the popover-based date picker recipe.
fn datetime_card(ui: &mut egui::Ui, _tokens: Tokens, state: &mut State) {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    Card::new()
        .title("Date")
        .description("An inline calendar and a popover date picker.")
        .show(ui, |ui| {
            ui.add_space(8.0);

            ui.vertical_centered(|ui| {
                Calendar::new(&mut state.selected_date)
                    .today(Date::new(2025, 6, 24))
                    .show(ui);
            });

            ui.add_space(8.0);
            let picked = state.selected_date.map_or_else(
                || "No date selected".to_owned(),
                |d| {
                    format!(
                        "Selected: {} {}, {}",
                        MONTHS[d.month as usize - 1],
                        d.day,
                        d.year
                    )
                },
            );
            Typography::new(picked, typography::Variant::Muted)
                .size(13.0)
                .ui(ui);

            ui.add_space(16.0);
            Separator::new().ui(ui);
            ui.add_space(16.0);

            // Date Picker: a Calendar in a Popover, triggered by an outline row.
            Typography::new("Date Picker", typography::Variant::Small)
                .size(13.0)
                .ui(ui);
            ui.add_space(6.0);
            glazier::DatePicker::new(&mut state.picker_date)
                .today(Date::new(2025, 6, 24))
                .placeholder("Pick a date")
                .show(ui);

            ui.add_space(16.0);
            Separator::new().ui(ui);
            ui.add_space(16.0);

            // Date Picker with Input: a typeable field + calendar icon-button.
            Typography::new("Date Picker with Input", typography::Variant::Small)
                .size(13.0)
                .ui(ui);
            ui.add_space(6.0);
            glazier::DatePicker::new(&mut state.input_date)
                .with_input(&mut state.input_date_text)
                .today(Date::new(2025, 6, 24))
                .placeholder("June 24, 2025")
                .show(ui);

            ui.add_space(16.0);
            Separator::new().ui(ui);
            ui.add_space(16.0);

            // Date and Time: a date picker beside a time field (shadcn group).
            Typography::new("Date and Time", typography::Variant::Small)
                .size(13.0)
                .ui(ui);
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                let half = (ui.available_width() - 12.0) / 2.0;
                glazier::DatePicker::new(&mut state.selected_date)
                    .today(Date::new(2025, 6, 24))
                    .placeholder("Pick a date")
                    .width(half)
                    .id_salt("datetime-date")
                    .show(ui);
                glazier::TimePicker::new(&mut state.picker_time)
                    .width(half)
                    .show(ui);
            });
        });
}

/// Sonner [`Toast`] triggers: a position picker (six anchors) plus variant and
/// stacking buttons. Drained by the app-level `Toaster`; fire a few and hover
/// the stack to watch it expand. Lives in the Overlays card.
fn toast_triggers(ui: &mut egui::Ui, _tokens: Tokens, state: &mut State) {
    use glazier::sonner::Corner;
    // Position picker — the six Sonner anchors. Each re-anchors the
    // stack and fires a toast there so you can see it land.
    let positions = [
        ("Top Left", Corner::TopLeft),
        ("Top Center", Corner::TopCenter),
        ("Top Right", Corner::TopRight),
        ("Bottom Left", Corner::BottomLeft),
        ("Bottom Center", Corner::BottomCenter),
        ("Bottom Right", Corner::BottomRight),
    ];
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for (label, corner) in positions {
            let active = state.toast_corner == corner;
            let variant = if active {
                Variant::Secondary
            } else {
                Variant::Outline
            };
            if Button::new(label)
                .variant(variant)
                .size(Size::Small)
                .ui(ui)
                .clicked()
            {
                state.toast_corner = corner;
                Toast::new("Event created")
                    .description("Sunday, June 12 at 9:00 AM")
                    .position(corner)
                    .send(ui.ctx());
            }
        }
    });

    ui.add_space(12.0);

    // Variant + stacking triggers — each enqueues into the app-wide queue.
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        if Button::new("Default")
            .variant(Variant::Outline)
            .size(Size::Small)
            .ui(ui)
            .clicked()
        {
            Toast::new("Event created")
                .description("Sunday, June 12 at 9:00 AM")
                .send(ui.ctx());
        }
        if Button::new("Success")
            .variant(Variant::Outline)
            .size(Size::Small)
            .ui(ui)
            .clicked()
        {
            Toast::new("Saved").success().send(ui.ctx());
        }
        if Button::new("Info")
            .variant(Variant::Outline)
            .size(Size::Small)
            .ui(ui)
            .clicked()
        {
            Toast::new("Heads up")
                .description("A new version is available.")
                .info()
                .send(ui.ctx());
        }
        if Button::new("Warning")
            .variant(Variant::Outline)
            .size(Size::Small)
            .ui(ui)
            .clicked()
        {
            Toast::new("Low disk space")
                .description("Only 2% remaining.")
                .warning()
                .send(ui.ctx());
        }
        if Button::new("Error")
            .variant(Variant::Outline)
            .size(Size::Small)
            .ui(ui)
            .clicked()
        {
            Toast::new("Could not save")
                .description("Check your connection and try again.")
                .error()
                .send(ui.ctx());
        }
        if Button::new("Stack ×3")
            .variant(Variant::Outline)
            .size(Size::Small)
            .ui(ui)
            .clicked()
        {
            Toast::new("Deploy queued").info().send(ui.ctx());
            Toast::new("Build started")
                .description("Compiling 248 modules…")
                .send(ui.ctx());
            Toast::new("Tests passed").success().send(ui.ctx());
        }
    });

    ui.add_space(8.0);
    Typography::new(
        "Pick a position, then hover the stack to expand it.",
        typography::Variant::Muted,
    )
    .size(12.0)
    .ui(ui);
}

// --- Small visual helpers (stand-ins for not-yet-built components) ----------

/// A label + [`Switch`] row.
fn toggle_row(ui: &mut egui::Ui, _tokens: Tokens, label: &str, on: &mut bool) {
    ui.horizontal(|ui| {
        Typography::p(label).size(13.0).ui(ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            Switch::new(on).ui(ui);
        });
    });
}

/// A label + value row.
fn stat_row(ui: &mut egui::Ui, _tokens: Tokens, label: &str, value: &str) {
    ui.horizontal(|ui| {
        // Muted caption (`text-sm text-muted-foreground`).
        Typography::new(label, typography::Variant::Muted)
            .size(13.0)
            .ui(ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Semibold foreground value (`text-sm font-medium`).
            Typography::new(value, typography::Variant::Small)
                .size(13.0)
                .ui(ui);
        });
    });
}

/// A faithful port of shadcn's `Message` / `MessageScroller` / `Marker` trio: a
/// chat transcript that follows the live edge as you send, anchors new turns
/// near the top, and shows a jump-to-latest button when you scroll away.
fn conversation_card(ui: &mut egui::Ui, state: &mut State) {
    Card::new()
        .title("Conversation")
        .description("Message, Marker & MessageScroller")
        .show(ui, |ui| {
            ui.add_space(4.0);

            // The transcript lives in a height-constrained MessageScroller.
            let mut scroller = MessageScroller::new("chat-demo")
                .auto_scroll(true)
                .default_position(Position::End)
                .previous_item_peek(48.0)
                .max_height(280.0);

            // A labelled day divider opens the thread.
            scroller = scroller.item("day", false, |ui| {
                Marker::new("Today")
                    .variant(marker::Variant::Separator)
                    .ui(ui);
            });

            for msg in &state.chat {
                let ChatMsg { id, mine, text } = msg.clone();
                // The user's own messages start each new turn -> anchor them.
                scroller = scroller.item(id, mine, move |ui| {
                    if mine {
                        Message::new()
                            .align(MsgSide::End)
                            .avatar(Avatar::new("Me").diameter(28.0))
                            .footer("Delivered")
                            .show(ui, |ui| {
                                ui.label(&text);
                            });
                    } else {
                        Message::new()
                            .avatar(Avatar::new("Riley").diameter(28.0))
                            .header("Riley")
                            .show(ui, |ui| {
                                ui.label(&text);
                            });
                    }
                });
            }

            // A streaming status marker pinned to the live edge.
            scroller = scroller.item("typing", false, |ui| {
                Marker::new("Riley is typing\u{2026}").shimmer(true).ui(ui);
            });

            scroller.show(ui);

            ui.add_space(8.0);

            // Composer: an input + send button. Sending appends a user turn,
            // which the scroller anchors near the top and then follows. Lay it
            // out right-to-left so the button takes its natural width first and
            // the input fills the remainder without overflowing the card.
            let mut send = false;
            let mut clicked = false;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                clicked = Button::new("Send").size(Size::Small).ui(ui).clicked();
                ui.add_space(8.0);
                let resp = InputGroup::new(&mut state.chat_draft)
                    .placeholder("Message\u{2026}")
                    .width(ui.available_width())
                    .ui(ui);
                send = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
            {
                if (send || clicked) && !state.chat_draft.trim().is_empty() {
                    state.chat.push(ChatMsg {
                        id: state.chat_seq,
                        mine: true,
                        text: std::mem::take(&mut state.chat_draft),
                    });
                    state.chat_seq += 1;
                }
            }
        });
}

/// A long option list for the timezone [`Combobox`] — exercises the panel's
/// height cap + scroll, and shows how the search field tames a big list.
const TIMEZONES: [&str; 24] = [
    "UTC",
    "Pacific/Honolulu",
    "America/Anchorage",
    "America/Los_Angeles",
    "America/Denver",
    "America/Chicago",
    "America/New_York",
    "America/Sao_Paulo",
    "Atlantic/Reykjavik",
    "Europe/Lisbon",
    "Europe/London",
    "Europe/Madrid",
    "Europe/Paris",
    "Europe/Berlin",
    "Europe/Athens",
    "Europe/Moscow",
    "Asia/Dubai",
    "Asia/Karachi",
    "Asia/Kolkata",
    "Asia/Bangkok",
    "Asia/Shanghai",
    "Asia/Tokyo",
    "Australia/Sydney",
    "Pacific/Auckland",
];

/// shadcn's `Command` showcase: a standalone command palette — a search field
/// over grouped, keyboard-navigable actions (↑/↓ to move, Enter to run), each
/// with an icon and an optional shortcut chip. The activated id is echoed below.
fn command_card(ui: &mut egui::Ui, tokens: Tokens, state: &mut State) {
    Card::new()
        .title("Command")
        .description("Command palette — search, Up/Down navigate, Enter to run")
        .show(ui, |ui| {
            // A bordered surface hosting the palette (the Combobox/Dialog reuse
            // this same engine).
            let frame = egui::Frame::new()
                .stroke(egui::Stroke::new(1.0, tokens.border))
                .corner_radius(tokens.radius_2xl())
                .inner_margin(egui::Margin::same(4));
            frame.show(ui, |ui| {
                if let Some(id) = Command::new("gallery-command")
                    .placeholder("Type a command or search…")
                    .width(ui.available_width())
                    .group(
                        CommandGroup::new("Suggestions").items([
                            CommandItem::new("calendar", "Calendar")
                                .icon(Icon::new(lucide::CALENDAR)),
                            CommandItem::new("search-emoji", "Search Emoji")
                                .icon(Icon::new(lucide::SEARCH)),
                            CommandItem::new("investments", "Investments")
                                .icon(Icon::new(lucide::INVESTMENTS)),
                        ]),
                    )
                    .group(
                        CommandGroup::new("Settings").items([
                            CommandItem::new("profile", "Profile")
                                .icon(Icon::new(lucide::DOCUMENTS))
                                .shortcut("Ctrl P")
                                .keywords(["account", "user"]),
                            CommandItem::new("billing", "Billing")
                                .icon(Icon::new(lucide::REPEAT))
                                .shortcut("Ctrl B"),
                            CommandItem::new("settings", "Settings")
                                .icon(Icon::new(lucide::SETTINGS))
                                .shortcut("Ctrl S"),
                        ]),
                    )
                    .show(ui)
                {
                    state.command_last = Some(id);
                }
            });

            ui.add_space(8.0);
            let status = state.command_last.as_deref().map_or_else(
                || "No command run yet.".to_owned(),
                |id| format!("Ran: {id}"),
            );
            Typography::new(status, typography::Variant::Muted)
                .size(13.0)
                .ui(ui);
        });
}

/// shadcn's `Menubar` showcase: a desktop-style row of dropdown menus (File,
/// Edit, View, Profiles). Click a trigger to open it; while a menu is open,
/// hovering a sibling switches to it (native menubar behaviour). The last
/// chosen "Menu › Item" is echoed below.
fn menubar_card(ui: &mut egui::Ui, tokens: Tokens, state: &mut State) {
    Card::new()
        .title("Menubar")
        .description("App-style menu bar — click to open, hover to switch")
        .show(ui, |ui| {
            // The bar sits in a bordered surface, like shadcn's `Menubar` shell.
            let frame = egui::Frame::new()
                .stroke(egui::Stroke::new(1.0, tokens.border))
                .corner_radius(tokens.radius_2xl())
                .inner_margin(egui::Margin::symmetric(4, 2));
            frame.show(ui, |ui| {
                let picked = Menubar::new("gallery-menubar")
                    .menu(MenubarMenu::new(
                        "File",
                        DropdownMenu::new()
                            .item("New Tab")
                            .item("New Window")
                            .separator()
                            .item("Share")
                            .separator()
                            .item("Print"),
                    ))
                    .menu(MenubarMenu::new(
                        "Edit",
                        DropdownMenu::new()
                            .item("Undo")
                            .item("Redo")
                            .separator()
                            .item("Cut")
                            .item("Copy")
                            .item("Paste"),
                    ))
                    .menu(MenubarMenu::new(
                        "View",
                        DropdownMenu::new()
                            .item("Reload")
                            .item("Toggle Fullscreen")
                            .separator()
                            .item("Hide Sidebar"),
                    ))
                    .menu(MenubarMenu::new(
                        "Profiles",
                        DropdownMenu::new()
                            .label("Switch account")
                            .item("Andy")
                            .item("Benoit")
                            .separator()
                            .destructive("Sign Out"),
                    ))
                    .show(ui);
                if let Some((menu, item)) = picked {
                    // Re-derive readable labels for the status line.
                    let menus = ["File", "Edit", "View", "Profiles"];
                    let items: [&[&str]; 4] = [
                        &["New Tab", "New Window", "Share", "Print"],
                        &["Undo", "Redo", "Cut", "Copy", "Paste"],
                        &["Reload", "Toggle Fullscreen", "Hide Sidebar"],
                        &["Andy", "Benoit", "Sign Out"],
                    ];
                    let label = items[menu].get(item).copied().unwrap_or("?");
                    state.menubar_last = Some(format!("{} › {label}", menus[menu]));
                }
            });

            ui.add_space(8.0);
            let status = state.menubar_last.as_deref().map_or_else(
                || "No menu item chosen yet.".to_owned(),
                |s| format!("Chose: {s}"),
            );
            Typography::new(status, typography::Variant::Muted)
                .size(13.0)
                .ui(ui);
        });
}

/// shadcn's `NavigationMenu` showcase: a site-header nav where each trigger
/// reveals a rich flyout panel of link cards. Hover-driven — move onto a
/// trigger to open, switch instantly between menus, travel onto the panel to
/// click. The last chosen link is echoed below.
fn nav_menu_card(ui: &mut egui::Ui, _tokens: Tokens, state: &mut State) {
    Card::new()
        .title("Navigation Menu")
        .description("Site nav with flyout panels — hover to open, click a link")
        .show(ui, |ui| {
            // Each menu's links, reused for both the panel body and the status.
            let getting_started: [(&str, &str); 3] = [
                ("Introduction", "Re-usable components built with egui."),
                ("Installation", "How to install and set up glazier."),
                ("Typography", "Styles for headings, prose and code."),
            ];
            let components: [(&str, &str); 4] = [
                ("Menubar", "App-style menu bar with dropdowns."),
                ("Command", "Fast, composable command palette."),
                ("Combobox", "Searchable single-select."),
                ("Hover Card", "Rich preview on hover."),
            ];

            let resp = NavigationMenu::new("gallery-nav")
                .item(NavItem::menu("Getting Started").panel_width(320.0))
                .item(NavItem::menu("Components").panel_width(380.0))
                .item(NavItem::link("Docs"))
                .show(ui, |idx, ui| {
                    let links: &[(&str, &str)] = if idx == 0 {
                        &getting_started
                    } else {
                        &components
                    };
                    for (title, desc) in links {
                        let clicked = Item::new()
                            .interactive(("nav-link", idx, *title))
                            .show(ui, |ui| {
                                Typography::new(*title, typography::Variant::Small)
                                    .size(14.0)
                                    .ui(ui);
                                Typography::new(*desc, typography::Variant::Muted)
                                    .size(12.0)
                                    .ui(ui);
                            })
                            .clicked();
                        if clicked {
                            state.nav_last = Some((*title).to_owned());
                            ui.close();
                        }
                    }
                });
            if let Some(i) = resp.link_clicked {
                state.nav_last = Some(["Getting Started", "Components", "Docs"][i].to_owned());
            }

            ui.add_space(8.0);
            let status = state.nav_last.as_deref().map_or_else(
                || "No link chosen yet.".to_owned(),
                |s| format!("Chose: {s}"),
            );
            Typography::new(status, typography::Variant::Muted)
                .size(13.0)
                .ui(ui);
        });
}

/// shadcn's Bubble showcase: the standalone bubble surface — a grouped thread
/// with start/end alignment and reactions overlapping the edge, plus a strip of
/// the seven visual variants.
fn bubble_card(ui: &mut egui::Ui, _tokens: Tokens, _state: &mut State) {
    Card::new()
        .title("Bubble")
        .description("Conversational surface — variants, alignment, reactions")
        .show(ui, |ui| {
            ui.add_space(6.0);
            ui.spacing_mut().item_spacing.y = 8.0;

            // A short grouped thread: a received bubble, then two sent ones with
            // a reaction row overlapping the edge.
            Bubble::new(BubbleVariant::Secondary).show(ui, |ui| {
                ui.label("Hey there! what's up?");
            });
            BubbleGroup::new().show(ui, |ui| {
                Bubble::new(BubbleVariant::Default)
                    .align(BubbleAlign::End)
                    .show(ui, |ui| {
                        ui.label("Want to see chat bubbles?");
                    });
                Bubble::new(BubbleVariant::Default)
                    .align(BubbleAlign::End)
                    .reactions(["\u{1f44d}", "\u{1f525}", "+2"])
                    .show(ui, |ui| {
                        ui.label("It can group, switch sides, and carry reactions.");
                    });
            });

            ui.add_space(6.0);
            Separator::new().ui(ui);
            ui.add_space(2.0);
            Typography::new("Variants", typography::Variant::Muted)
                .size(12.0)
                .ui(ui);

            // One bubble per variant so the treatments line up.
            for (variant, text) in [
                (BubbleVariant::Default, "Default — a strong primary bubble."),
                (BubbleVariant::Secondary, "Secondary — the neutral default."),
                (BubbleVariant::Muted, "Muted — quiet supporting content."),
                (BubbleVariant::Tinted, "Tinted — a soft primary wash."),
                (BubbleVariant::Outline, "Outline — a bordered bubble."),
                (BubbleVariant::Destructive, "Destructive — a failed action."),
                (
                    BubbleVariant::Ghost,
                    "Ghost — unframed assistant text, full width.",
                ),
            ] {
                Bubble::new(variant).show(ui, |ui| {
                    ui.label(text);
                });
            }
        });
}

/// shadcn's Chart showcase: a multi-series visitor chart that switches between
/// bar / line / area marks via a toggle group. Hovering a month floats a
/// tooltip card listing each series' value; a legend sits below the plot.
fn chart_card(ui: &mut egui::Ui, _tokens: Tokens, state: &mut State) {
    const MONTHS: [&str; 6] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];
    const DESKTOP: [f32; 6] = [186.0, 305.0, 237.0, 173.0, 209.0, 264.0];
    const MOBILE: [f32; 6] = [80.0, 200.0, 120.0, 190.0, 130.0, 140.0];
    Card::new()
        .title("Chart")
        .description("Bar / line / area — hover for a tooltip")
        .show(ui, |ui| {
            ToggleGroup::single(&mut state.chart_kind, ["Bar", "Line", "Area"]).ui(ui);
            ui.add_space(10.0);
            let kind = match state.chart_kind {
                1 => ChartKind::Line,
                2 => ChartKind::Area,
                _ => ChartKind::Bar,
            };
            Chart::new(kind)
                .labels(MONTHS)
                .series(Series::new("Desktop", DESKTOP))
                .series(Series::new("Mobile", MOBILE))
                .height(200.0)
                .show(ui);
        });
}

/// shadcn's collapsible `Sidebar` (collapsible="icon") showcase: an app-shell
/// panel that animates between a labelled rail and a slim icon rail. A pinned
/// header + footer bracket a scrollable list of collapse-aware nav rows; the
/// built-in trigger and edge rail both toggle it, and an external
/// [`SidebarTrigger`] in the card header drives the same state.
fn sidebar_card(ui: &mut egui::Ui, _tokens: Tokens, state: &mut State) {
    use egui::Widget as _;
    use glazier::sidebar::{Sidebar, SidebarTrigger};

    const NAV: [(&str, &str); 5] = [
        (lucide::ANALYTICS, "Dashboard"),
        (lucide::TRANSACTIONS, "Transactions"),
        (lucide::REPORTS, "Reports"),
        (lucide::PROFILE, "Profile"),
        (lucide::SETTINGS, "Settings"),
    ];

    Card::new()
        .title("Sidebar")
        .description("Collapsible app shell — trigger, rail, icon rail")
        .show(ui, |ui| {
            // External trigger living in the content header.
            ui.horizontal(|ui| {
                SidebarTrigger::new("gallery-sidebar").ui(ui);
                Typography::new("Toggle from anywhere", typography::Variant::Muted)
                    .size(13.0)
                    .ui(ui);
            });
            ui.add_space(8.0);

            Sidebar::new("gallery-sidebar")
                .height(264.0)
                .header(|ui, collapsed| {
                    sidebar_brand(ui, collapsed);
                    ui.add_space(6.0);
                })
                .footer(|ui, collapsed| {
                    ui.add_space(6.0);
                    sidebar_nav_row(ui, lucide::PROFILE, "Shadcn", collapsed, false);
                })
                .show(ui, |ui, collapsed| {
                    for (i, (icon, label)) in NAV.iter().enumerate() {
                        if sidebar_nav_row(ui, icon, label, collapsed, i == state.sidebar_nav) {
                            state.sidebar_nav = i;
                        }
                        ui.add_space(4.0);
                    }
                });
        });
}

/// The pinned brand row: an avatar-ish glyph tile and (when expanded) a title.
fn sidebar_brand(ui: &mut egui::Ui, collapsed: bool) {
    let tokens = Tokens::get(ui);
    let h = 36.0;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), h), egui::Sense::hover());
    let tile = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.center().y - 14.0),
        egui::Vec2::splat(28.0),
    );
    ui.painter()
        .rect_filled(tile, tokens.radius_md(), tokens.primary);
    glazier::icon::Icon::new(lucide::ANALYTICS)
        .size(16.0)
        .color(tokens.primary_foreground)
        .image(tokens)
        .paint_at(
            ui,
            egui::Rect::from_center_size(tile.center(), egui::Vec2::splat(16.0)),
        );
    if !collapsed {
        let galley = ui.painter().layout_no_wrap(
            "Acme Inc.".to_owned(),
            glazier::fonts::semibold(ui, 14.0),
            tokens.foreground,
        );
        ui.painter().galley(
            egui::pos2(tile.right() + 8.0, rect.center().y - galley.size().y / 2.0),
            galley,
            tokens.foreground,
        );
    }
}

/// A collapse-aware nav row: leading icon, accent fill when active/hovered, and
/// a label that hides (icon centres) when the rail is collapsed. Returns whether
/// it was clicked.
fn sidebar_nav_row(
    ui: &mut egui::Ui,
    icon: &'static str,
    label: &str,
    collapsed: bool,
    active: bool,
) -> bool {
    let tokens = Tokens::get(ui);
    let h = 32.0;
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), h), egui::Sense::click());
    let hover_t = ui
        .ctx()
        .animate_bool_with_time(resp.id.with("hov"), resp.hovered(), 0.15);
    let t = if active { 1.0 } else { hover_t };
    if t > 0.01 {
        ui.painter()
            .rect_filled(rect, tokens.radius_md(), tokens.accent.gamma_multiply(t));
    }
    let fg = tokens.foreground.lerp_to_gamma(tokens.accent_foreground, t);
    let isz = 16.0;
    let ix = if collapsed {
        rect.center().x - isz / 2.0
    } else {
        rect.left() + 8.0
    };
    let irect = egui::Rect::from_min_size(
        egui::pos2(ix, rect.center().y - isz / 2.0),
        egui::Vec2::splat(isz),
    );
    glazier::icon::Icon::new(icon)
        .size(isz)
        .color(fg)
        .image(tokens)
        .paint_at(ui, irect);
    if !collapsed {
        let galley =
            ui.painter()
                .layout_no_wrap(label.to_owned(), egui::FontId::proportional(13.0), fg);
        ui.painter().galley(
            egui::pos2(irect.right() + 8.0, rect.center().y - galley.size().y / 2.0),
            galley,
            fg,
        );
    }
    resp.clicked()
}

/// shadcn's Data Table showcase: a sortable, filterable, paginated table over
/// a small invoice dataset. Click a header to cycle sort; type in the filter to
/// narrow rows; page through with the controls below.
fn data_table_card(ui: &mut egui::Ui, _tokens: Tokens) {
    use glazier::data_table::{DataColumn, DataTable};
    use glazier::table::Sizing;

    const ROWS: [[&str; 4]; 9] = [
        ["INV001", "Paid", "Credit Card", "250.00"],
        ["INV002", "Pending", "PayPal", "150.00"],
        ["INV003", "Unpaid", "Bank Transfer", "350.00"],
        ["INV004", "Paid", "Credit Card", "450.00"],
        ["INV005", "Paid", "PayPal", "550.00"],
        ["INV006", "Pending", "Bank Transfer", "200.00"],
        ["INV007", "Unpaid", "Credit Card", "300.00"],
        ["INV008", "Paid", "PayPal", "125.00"],
        ["INV009", "Pending", "Credit Card", "475.00"],
    ];

    Card::new()
        .title("Data Table")
        .description("Sort, filter & paginate — click a header to sort")
        .show(ui, |ui| {
            let mut table = DataTable::new("invoices")
                .column(DataColumn::new("Invoice"))
                .column(DataColumn::new("Status"))
                .column(DataColumn::new("Method").sizing(Sizing::Remainder))
                .column(
                    DataColumn::new("Amount")
                        .align(egui::Align::RIGHT)
                        .numeric(),
                )
                .page_size(5)
                .search_placeholder("Filter invoices…");
            for r in ROWS {
                table = table.row(r);
            }
            table.show(ui);
        });
}

/// shadcn's prompt-kit Attachment showcase: a stack of staged-file chips above
/// a composer. Each chip infers its icon + `TYPE · SIZE` from the extension,
/// carries a remove button, and — below — shows the upload-state treatments
/// (a shimmering `uploading` title and a destructive `error` chip).
fn attachment_card(ui: &mut egui::Ui, _tokens: Tokens, state: &mut State) {
    use glazier::attachment::State as Upload;
    Card::new()
        .title("Attachment")
        .description("Staged files — icon, TYPE · SIZE, remove, upload state")
        .show(ui, |ui| {
            if state.attachments.is_empty() {
                Typography::new("No files attached.", typography::Variant::Muted)
                    .size(13.0)
                    .ui(ui);
            }
            let mut drop: Option<usize> = None;
            for (i, (name, bytes)) in state.attachments.iter().enumerate() {
                let r = Attachment::new(name.clone())
                    .size_bytes(*bytes)
                    .removable(true)
                    .width(280.0)
                    .show(ui);
                if r.removed {
                    drop = Some(i);
                }
                ui.add_space(6.0);
            }
            if let Some(i) = drop {
                state.attachments.remove(i);
            }
            if state.attachments.len() < 3 {
                ui.add_space(2.0);
                if Button::new("Reset list")
                    .variant(Variant::Outline)
                    .size(Size::Small)
                    .ui(ui)
                    .clicked()
                {
                    state.attachments = vec![
                        ("quarterly-report.pdf".to_owned(), 284_103),
                        ("hero-banner.png".to_owned(), 1_572_864),
                        ("voice-memo.m4a".to_owned(), 642_000),
                    ];
                }
            }

            ui.add_space(12.0);
            Typography::new("Upload states", typography::Variant::Muted)
                .size(12.0)
                .ui(ui);
            ui.add_space(4.0);
            let _ = Attachment::new("design-system.fig")
                .size_bytes(8_490_000)
                .state(Upload::Uploading)
                .width(280.0)
                .show(ui);
            ui.add_space(6.0);
            let _ = Attachment::new("render-output.mov")
                .description("Upload failed — file too large")
                .state(Upload::Error)
                .removable(true)
                .width(280.0)
                .show(ui);
        });
}

/// shadcn's `InputOTP` showcase: a segmented one-time-code field. Type to fill
/// cells left-to-right (auto-advance), Backspace to retreat, arrows/click to
/// move the caret. Split 3 + 3 with a separator. The live code is echoed below.
fn input_otp_card(ui: &mut egui::Ui, _tokens: Tokens, state: &mut State) {
    Card::new()
        .title("Input OTP")
        .description("Segmented code field — digits or letters, paste-aware")
        .show(ui, |ui| {
            ui.add_space(2.0);
            let done = ui
                .horizontal(|ui| {
                    InputOtp::new(&mut state.otp_code, 6)
                        .group(3)
                        .mode(OtpMode::Alphanumeric)
                        .id_salt("gallery-otp")
                        .show(ui)
                })
                .inner;

            ui.add_space(10.0);
            let status = if state.otp_code.is_empty() {
                "Enter your one-time passcode.".to_owned()
            } else if done || state.otp_code.chars().count() == 6 {
                format!("Code complete: {}", state.otp_code)
            } else {
                format!("… {}", state.otp_code)
            };
            Typography::new(status, typography::Variant::Muted)
                .size(13.0)
                .ui(ui);
        });
}

/// A faithful port of shadcn's Typography page: the heading/prose scale
/// (`h1`…`muted`, blockquote, list, inline code), topped with a [`NativeSelect`]
/// — the bordered, native-style picker — choosing which heading style leads.
fn typography_card(ui: &mut egui::Ui, _tokens: Tokens, state: &mut State) {
    Card::new()
        .title("Typography")
        .description("Prose scale, NativeSelect, Combobox & HoverCard")
        .show(ui, |ui| {
            // NativeSelect: pick the lead heading variant.
            let heads = ["h1", "h2", "h3", "h4"];
            ui.horizontal(|ui| {
                Typography::new("Lead style", typography::Variant::Muted)
                    .size(13.0)
                    .ui(ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    NativeSelect::new(&mut state.type_size, heads)
                        .width(120.0)
                        .show(ui);
                });
            });
            ui.add_space(8.0);

            // Combobox: a searchable single-select (Command in a Popover).
            ui.horizontal(|ui| {
                Typography::new("Framework", typography::Variant::Muted)
                    .size(13.0)
                    .ui(ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    Combobox::new(
                        &mut state.combo_framework,
                        [
                            "Next.js",
                            "SvelteKit",
                            "Nuxt.js",
                            "Remix",
                            "Astro",
                            "Gatsby",
                        ],
                    )
                    .id_salt("combo-framework")
                    .placeholder("Select framework…")
                    .search_placeholder("Search framework…")
                    .width(200.0)
                    .show(ui);
                });
            });
            ui.add_space(8.0);

            // Combobox over a long list — the panel caps its height and scrolls,
            // and the search field filters it down fast.
            ui.horizontal(|ui| {
                Typography::new("Timezone", typography::Variant::Muted)
                    .size(13.0)
                    .ui(ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    Combobox::new(&mut state.combo_timezone, TIMEZONES)
                        .id_salt("combo-timezone")
                        .placeholder("Select timezone…")
                        .search_placeholder("Search timezone…")
                        .width(200.0)
                        .show(ui);
                });
            });
            ui.add_space(10.0);

            let lead = match state.type_size {
                0 => typography::Variant::H1,
                2 => typography::Variant::H3,
                3 => typography::Variant::H4,
                _ => typography::Variant::H2,
            };
            Typography::new("The Joke Tax", lead).ui(ui);
            ui.add_space(6.0);

            Typography::new(
                "The king thought long and hard, and finally came up with a brilliant plan.",
                typography::Variant::Lead,
            )
            .ui(ui);
            ui.add_space(8.0);

            Typography::p(
                "The people of the kingdom paid the joke tax — a tax on every joke told.",
            )
            .ui(ui);
            ui.add_space(8.0);

            Typography::new(
                "After all, what is a kingdom without a little humor?",
                typography::Variant::Blockquote,
            )
            .ui(ui);
            ui.add_space(10.0);

            Typography::list([
                "1st level of puns: 5 gold coins",
                "2nd level of jokes: 10 gold coins",
                "3rd level of one-liners: 20 gold coins",
            ])
            .ui(ui);
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                Typography::new("npx", typography::Variant::InlineCode).ui(ui);
                ui.add_space(6.0);
                Typography::new("shadcn add", typography::Variant::Small).ui(ui);
            });
            ui.add_space(6.0);
            Typography::new("A small footnote in muted ink.", typography::Variant::Muted).ui(ui);
            ui.add_space(10.0);

            // HoverCard: a link-style trigger revealing a profile preview that
            // stays open while the pointer is over the card.
            ui.horizontal(|ui| {
                Typography::new("Author", typography::Variant::Muted)
                    .size(13.0)
                    .ui(ui);
                ui.add_space(6.0);
                let trigger = Button::new("@peduarte").variant(Variant::Link).ui(ui);
                HoverCard::new().width(280.0).show(ui, &trigger, |ui| {
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::splat(44.0), egui::Sense::hover());
                        Avatar::new("Pedro Duarte")
                            .diameter(44.0)
                            .paint_at(ui, rect);
                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            Typography::new("Pedro Duarte", typography::Variant::Small)
                                .size(14.0)
                                .ui(ui);
                            Typography::new("@peduarte", typography::Variant::Muted)
                                .size(13.0)
                                .ui(ui);
                        });
                    });
                    ui.add_space(8.0);
                    Typography::new(
                        "Created shadcn/ui. Building tools that make the web \
                         feel handcrafted.",
                        typography::Variant::Muted,
                    )
                    .size(13.0)
                    .ui(ui);
                    ui.add_space(8.0);
                    Typography::new("Joined December 2021", typography::Variant::Muted)
                        .size(12.0)
                        .ui(ui);
                });
            });
        });
}

/// A labelled [`Progress`] bar row.
fn progress_row(ui: &mut egui::Ui, _tokens: Tokens, label: &str, fraction: f32) {
    ui.horizontal(|ui| {
        Typography::p(label).size(13.0).ui(ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            Typography::new(
                format!("{:.0}%", fraction * 100.0),
                typography::Variant::Muted,
            )
            .size(12.0)
            .ui(ui);
        });
    });
    ui.add_space(6.0);
    Progress::new(fraction).ui(ui);
}

/// A bar chart from a list of 0..1 heights, each bar `chart-2` with a rounded
/// top and a muted month label centered beneath it.
#[allow(clippy::cast_precision_loss)] // bar counts are tiny
fn bar_chart(ui: &mut egui::Ui, tokens: Tokens, values: &[f32], labels: &[&str]) {
    const BAR_AREA: f32 = 180.0; // chart body height
    const LABEL_H: f32 = 20.0; // row reserved for month labels
    const MIN_BAR: f32 = 8.0; // shadcn `min-h-2`

    let color = chart_2(ui.visuals().dark_mode);
    let (rect, _) = ui.allocate_at_least(
        Vec2::new(ui.available_width(), BAR_AREA + LABEL_H),
        egui::Sense::hover(),
    );
    let n = values.len();
    let gap = 12.0; // gap-3
    let bar_w = (rect.width() - gap * (n as f32 - 1.0)) / n as f32;
    // `rounded-t-md`: round the top corners only.
    let r = tokens.radius_md();
    let top_round = egui::CornerRadius {
        nw: r,
        ne: r,
        sw: 0,
        se: 0,
    };
    let baseline = rect.top() + BAR_AREA;

    for (i, v) in values.iter().enumerate() {
        let x = (i as f32).mul_add(bar_w + gap, rect.left());
        let h = (BAR_AREA * v.clamp(0.0, 1.0)).max(MIN_BAR);
        let bar = egui::Rect::from_min_size(egui::pos2(x, baseline - h), Vec2::new(bar_w, h));
        ui.painter().rect_filled(bar, top_round, color);

        if let Some(label) = labels.get(i) {
            ui.painter().text(
                egui::pos2(x + bar_w / 2.0, baseline + LABEL_H / 2.0),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(12.0),
                tokens.muted_foreground,
            );
        }
    }
}
