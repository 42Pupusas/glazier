//! glazier — composable, style-driven decorators for egui widgets.
//!
//! The core idea: every [`egui::Widget`] automatically gains chainable
//! decorator methods via the [`Decorate`] blanket trait. Import the trait and
//! the methods light up on *all* your components — no per-type boilerplate.
//!
//! Decorators read their defaults from [`egui::Visuals`]/[`egui::Style`], so
//! when a user overrides their app style, every glazier component responds.
//!
//! ```no_run
//! use glazier::{ChatBubble, Decorate};
//! use egui::Widget as _;
//! # egui::__run_test_ui(|ui| {
//! ChatBubble::new("hello").carded().rounded().padded(8.0).ui(ui); // one Frame
//! # });
//! ```
//!
//! [`Decorate`] handles structural wrapping (card, rounding, padding); its
//! sibling [`Customize`] handles *paint*. Where a component has a variant- and
//! theme-driven look that goes deeper than a wrapping frame, it exposes its
//! real, internal resolved style (e.g. [`button::ButtonStyle`], or
//! [`egui::Frame`] itself for anything Frame-backed) through one
//! `.style(|s| ...)` hook — see [`Customize`] for why this replaced a
//! per-property override on every component.

pub mod components;
mod customize;
mod decorate;
pub mod fonts;
pub mod tokens;
mod widgets;

pub use components::{
    accordion, alert, alert_dialog, aspect_ratio, attachment, avatar, badge, breadcrumb, bubble,
    button, button_group, calendar, card, carousel, chart, checkbox, collapsible, combobox,
    command, context_menu, data_table, date_picker, dialog, drawer, dropdown_menu, empty, field,
    grid, hover_card, icon, input, input_group, input_otp, item, kbd, label, marker, menubar,
    message, message_scroller, native_select, navigation_menu, pagination, popover, progress,
    radio_group, resizable, scroll_area, select, separator, sheet, sidebar, sidebar_menu, skeleton,
    slider, sonner, spinner, switch, table, tabs, textarea, time_picker, toggle, toggle_group,
    tooltip, typography, Accordion, Alert, AlertDialog, AspectRatio, Attachment, Avatar, Badge, BadgeStyle,
    Breadcrumb, Bubble, BubbleGroup, Button, ButtonGroup, ButtonStyle, Calendar, Card, Carousel, Chart,
    Checkbox, Collapsible, Combobox, Command, CommandGroup, CommandItem, ContextMenu, DataColumn,
    DataTable, Date, DatePicker, Dialog, Drawer, DropdownMenu, Empty, Field, Grid, HoverCard, Icon,
    Input, InputGroup, InputOtp, Item, Kbd, Label, Marker, Menubar, MenubarMenu, Message,
    MessageGroup, MessageResponse, MessageScroller, NativeSelect, NavItem, NavResponse, NavigationMenu, OtpMode,
    Pagination, Popover, Progress, RadioGroup, Resizable, ScrollArea, Select, Separator, Sheet,
    Sidebar, SidebarMenu, SidebarTrigger, Sizing, Skeleton, Slider, Spinner, Switch, Table, Tabs,
    TabsResponse, Textarea, Time, TimePicker, Toast, Toaster, Toggle, ToggleGroup, Tooltip, Typography,
};
pub use customize::{Customize, StyleHook};
pub use decorate::{Decorate, Styled};
pub use fonts::install as install_fonts;
pub use tokens::{apply_style, shadcn_visuals, shadcn_visuals_from, Tokens};
pub use widgets::ChatBubble;
