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
//!
//! A third sibling, [`Sizeable`], does the same thing for *geometry* — the
//! row heights, paddings, gaps, icon sizes and animation durations that used
//! to be hardcoded `const`s. One `.sizing(|m| ...)` hook per component, over
//! a plain `Default`-implementing metrics struct.

pub mod components;
mod customize;
mod decorate;
pub mod fonts;
mod sizing;
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
    tooltip, typography, Accordion, AccordionMetrics, Alert, AlertDialog, AlertDialogMetrics,
    AlertMetrics, AspectRatio, Attachment, AttachmentMetrics, Avatar, Badge, BadgeMetrics,
    BadgeStyle, Breadcrumb, BreadcrumbMetrics, Bubble, BubbleGroup, Button, ButtonGroup,
    ButtonGroupMetrics, ButtonMetrics, ButtonStyle, Calendar, CalendarMetrics, Card, Carousel,
    CarouselMetrics, Chart, Checkbox, CheckboxMetrics, Collapsible, CollapsibleMetrics, Combobox,
    ComboboxMetrics, Command, CommandGroup, CommandItem, ContextMenu, DataColumn, DataTable,
    DataTableMetrics, Date, DatePicker, DatePickerMetrics, Dialog, Drawer, DrawerMetrics,
    DropdownMenu, DropdownMenuMetrics, Empty, EmptyMetrics, Field, Grid, HoverCard,
    HoverCardMetrics, Icon, Input, InputGroup, InputGroupMetrics, InputMetrics, InputOtp,
    InputOtpMetrics, Item, Kbd, Label, Marker, MarkerMetrics, MarkerStyle, Menubar, MenubarMenu,
    MenubarMetrics, Message, MessageGroup, MessageResponse, MessageScroller,
    MessageScrollerMetrics, NativeSelect, NativeSelectMetrics, NavItem, NavResponse,
    NavigationMenu, OtpMode, Pagination, PaginationMetrics, Popover, Progress, RadioGroup,
    RadioGroupMetrics, Resizable, ResizableMetrics, ResizableStyle, ScrollArea, Select,
    SelectMetrics, Separator, Sheet, SheetMetrics, Sidebar, SidebarMenu, SidebarMenuMetrics,
    SidebarTrigger, Sizing, Skeleton, Slider, SliderMetrics, Spinner, SpinnerMetrics, SpinnerStyle,
    Switch, SwitchMetrics, Table, TableMetrics, Tabs, TabsMetrics, TabsResponse, Textarea, Time,
    TimePicker, TimePickerMetrics, Toast, Toaster, Toggle, ToggleGroup, ToggleGroupMetrics,
    ToggleMetrics, Tooltip, TooltipMetrics, Typography, TypographyMetrics,
};
pub use customize::{Customize, StyleHook};
pub use decorate::{Decorate, Styled};
pub use fonts::install as install_fonts;
pub use sizing::{Sizeable, SizingHook};
pub use tokens::{apply_style, shadcn_visuals, shadcn_visuals_from, Tokens};
pub use widgets::ChatBubble;
