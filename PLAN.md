# glazier — shadcn/ui component plan

Goal: mock the [shadcn/ui](https://ui.shadcn.com) component set in egui 0.34, each
built as a plain `Widget` (no `*mut App`) and styled through glazier's `Decorate`
chain so a user's `Style`/`Visuals` override flows through everything.

Tiers are ordered by build difficulty. We ship one component at a time, compiling
clean (clippy pedantic+nursery) after each.

---

## Customization audit (2026-06-30)

While chasing full shadcn coverage, several components grew **one-off escape
hatches**: `Spinner::color`, `Marker::color`, `Badge::dot`, `Message::bubble_fill`
— each a bespoke `Option<Color32>` field + builder method covering whatever
sliver of paint the author happened to expose that day. That's not a strategy,
it's guesswork repeated per component, and it can never cover the field nobody
guessed.

**Rejected approach:** a `Fill`/`Outline`/`TextColor`/`Radius` blanket-trait set,
one `Option<T>` per property, implemented per component. Tried it, reverted it —
it's the *same* escape-hatch sprawl wearing a shared name. Still only covers
the properties the trait author enumerated.

**Adopted approach: [`Customize<T>`](src/customize.rs).** Every component that
paints a surface already computes a **real, internal style value** from its
`Variant` + the active `Tokens` before drawing — `Button` resolves a `Paint`,
`Bubble` resolves a `Style`, most framed components build an `egui::Frame`
directly. `Customize<T>` is one trait, one method (`.style(FnOnce(&mut T))`),
that:

1. Makes that internal record **public** (`ButtonStyle`, `BadgeStyle`, …) —
   for components already built straight out of `egui::Frame` (`Card`,
   `Alert`, `Popover`, …), `T` is `egui::Frame` itself, egui's own
   field-complete style type, no new struct needed.
2. Lets the caller's closure run against that value **after** the variant's
   defaults are resolved and **before** it's painted — full access to every
   field that exists, none hidden, none guessed.
3. Is the same shape everywhere: one field (`style_hook: StyleHook<T>`), one
   `impl Customize<T>`, one line in `Widget::ui`/`show` to `.apply()` it. The
   `Decorate` trait already proved this pattern (one blanket trait, uniform
   method name, identical everywhere) for *structural* wrapping; `Customize`
   is the same idea for *paint*.

**Reference implementations:** `Button`/`ButtonStyle`, `Badge`/`BadgeStyle`
(non-Frame, hand-painted), `Card`/`egui::Frame` (Frame-backed). Both shapes
are exercised in `src/customize.rs`'s doctest.

**Migration status (2026-07-01): done.** Every component named in the
original backlog now implements `Customize`, plus a few more found along the
way:

- Hand-painted style structs: `Spinner`/`SpinnerStyle`, `Marker`/`MarkerStyle`,
  `Message`/`BubbleStyle` (message.rs), `Bubble`/`BubbleStyle` (bubble.rs, its
  own resolved fill/text/stroke/framed/full_width record).
- Frame-backed (`T = egui::Frame`): `Alert`, `Popover`, `HoverCard`, `Item`
  (non-interactive path only — an `interactive` item paints its own
  hover-eased fill with no single resolved frame to hand back), `Empty`,
  `ScrollArea` (only takes effect when `bordered`), `Input`, `InputGroup`,
  `Textarea`, `Tooltip`, `Dialog`, `AlertDialog`, `Sheet`, `Drawer` (all four
  thread the hook through the shared `dialog::modal_shell`), `NavigationMenu`
  (the flyout panel frame), `SidebarMenu` (the group's outer padding frame).
- **Finished 2026-07-01 (was left half-migrated):** `Resizable`/`ResizableStyle`
  (divider hairline colour + hover/rest grip colours). It got the `Sizeable`
  pass in sizing batch 1 but was skipped for `Customize` and kept a bespoke
  `divider_color: Option<Color32>` builder — exactly the one-off escape hatch
  this trait replaced everywhere else. Folded that single field into
  `ResizableStyle` alongside the grip colours (previously not overridable at
  all) and dropped the old builder method; downstream callers use
  `.style(|s: &mut ResizableStyle| s.divider = ...)` instead.
- `Badge::dot` stayed bespoke on purpose, as originally called out — it's a
  genuinely distinct sub-element (a status indicator), not the badge's own
  surface.
- **Not migrated, deliberately:** `Sonner`/`Toast`. A `Toast` is enqueued into
  a `Vec<LiveToast>` in ctx memory and rendered later by `Toaster::show`,
  which paints a *variable number* of cards per call from data, not a single
  resolved value a one-shot `FnOnce` hook could target. Revisit if `StyleHook`
  ever grows an `Fn`-based repeatable variant; not worth forcing today.
- `Chart::color` is a per-series override already covering the one paintable
  property a `Series` has (there's no broader chart "surface" style to
  generalize into — the grid/legend/tooltip colours are all derived from
  `Tokens` directly, by design, so a page's chart always matches its theme).
  Left as-is; revisit only if a real need for overriding those surfaces shows up.

Each migration was a ~10-line change: add a style_hook field to the struct, impl Customize<T>, and one style_hook.apply(&mut resolved) call right before painting (via std::mem::take where the paint path needed &mut self afterward, since StyleHook::apply takes self by value).

---

## Sizing audit (2026-07-01)

Separate problem, same shape: components are full of small const f32/u8/usize values (row heights, paddings, gaps, icon sizes, animation durations) picked to match shadcn's Tailwind scale. They're sane defaults, not hard limits -- a caller with a denser layout or a different icon set has no way to reach them. ~200 such constants exist across ~49 component files.

Adopted approach: Sizeable<T> (src/sizing.rs). Direct sibling of Customize<T>, same shape, for geometry instead of paint:

1. Each metrics-heavy component gets one struct enumerating its own constants as public Copy fields (BadgeMetrics, SliderMetrics, CollapsibleMetrics, ...), with a Default impl reproducing the exact numbers that used to be hardcoded consts.
2. One method, .sizing(FnOnce(&mut T)), hands the caller that struct after defaults are built and before layout runs.
3. One free function, sizing::resolve(hook) -> T, is the component-side one-liner: build T::default(), apply the pending hook, hand back the resolved metrics to lay out with.

Example: Badge::new("New").sizing(|m: &mut BadgeMetrics| m.icon_gap = 8.0).ui(ui);

Reference implementations: Badge/BadgeMetrics (sizes + gaps, no timing), Slider/SliderMetrics (sizes + one animation duration + a u8 radius field), Collapsible/CollapsibleMetrics (a component whose consts were split between module-level and a const block inside show() -- unified into one struct). All three compile clean, clippy clean, and are exercised by src/sizing.rs's own doctest plus each component's existing doctest.

**Migration status (2026-07-03): done.** Every component in the original backlog now implements `Sizeable`: badge, slider, collapsible (batch 1), plus date_picker, tabs, menubar, pagination, tooltip, button, alert, input_otp, hover_card, sidebar_menu, drawer, switch, spinner, checkbox, dropdown_menu, breadcrumb, alert_dialog, empty, carousel, sheet, select, message, navigation_menu, marker, time_picker, dialog, combobox, data_table, chart, bubble, command, radio_group, attachment, native_select, table, input_group, toggle_group, sonner, toggle, accordion, message_scroller, resizable, input, button_group, calendar, sidebar (batches 2-22, see git log). SVG path-data constants (icon glyphs) stayed out of scope -- those aren't sizing, they're the icon itself.

Two components needed a variant on the standard shape:
- **`DropdownMenu`** resolves its metrics *eagerly* in the builder (a plain `DropdownMenuMetrics` field, no deferred hook) rather than the usual `SizingHook<T>` + `resolve()` pattern, because `show_contents` takes `&self` — menu entries are borrowed out of a parent `Menubar`/`ButtonGroup`/`ContextMenu` rather than consumed by value, so there's no by-value call site left to resolve a hook against. `.sizing()` is overridden to apply the closure immediately while `self` is still owned; `Sizeable`'s mandatory `sizing_hook_mut` is kept only to satisfy the trait and is otherwise unused.
- **`Sonner`** (`Toaster`) resolves its metrics once per `Toaster::show` call (not per-toast) since `Toast`/`LiveToast` are plain data queued into ctx memory, not one-shot builders each carrying their own hook.

All batches compile clean (clippy pedantic+nursery, all workspace crates) and pass the full test + doctest suite.

---

## Tier 0 — primitives (foundation the rest lean on)
These are mostly thin wrappers over egui built-ins + glazier decorators. Fast wins
that establish the styling vocabulary (variants, sizes, tokens).

| # | Component   | Notes |
|---|-------------|-------|
| 1 | **Button**  | variants: default/secondary/destructive/outline/ghost/link; sizes sm/md/lg/icon |
| 2 | Badge       | variants: default/secondary/destructive/outline |
| 3 | Label       | text + optional `for` association |
| 4 | Separator   | horizontal/vertical, 1px hairline |
| 5 | Skeleton    | pulsing placeholder block |
| 6 | Card        | header/content/footer slots — already half-done via `.carded()` |
| 7 | Avatar      | image or initials fallback, circular clip |
| 8 | Aspect Ratio| fixed-ratio container |
| 9 | Kbd / Code  | inline mono chips |

## Tier 1 — simple interactive (single state value)
Self-contained controls bound to one `&mut` value.

| # | Component    | Notes |
|---|--------------|-------|
| 10| Input        | text field with shadcn border/focus ring |
| 11| Textarea     | multiline input |
| 12| Checkbox     | check + indeterminate |
| 13| Switch       | animated toggle track/thumb |
| 14| Radio Group  | mutually exclusive set |
| 15| Slider       | track + thumb, styled |
| 16| Toggle       | pressed/unpressed button |
| 17| Progress     | determinate bar |
| 18| Toggle Group | segmented set of toggles |

## Tier 2 — composite layout (multiple children / slots)
Containers that arrange child widgets; need layout logic but no overlay.

| # | Component    | Notes |
|---|--------------|-------|
| 19| Tabs         | tab list + panels, active underline |
| 20| Accordion    | collapsible sections |
| 21| Collapsible  | single expand/collapse |
| 22| Table        | header/rows, zebra, alignment — ✅ (Sizing enum, per-col align, hover) |
| 23| Breadcrumb   | path with separators |
| 24| Pagination   | page buttons + prev/next — ✅ (ellipsis collapse, prev/next) |
| 25| Alert        | icon + title + description, variants |
| 26| Scroll Area  | styled scrollbars |
| 27| Resizable    | draggable split panes |
| 28| Carousel     | horizontal snap scroller — ✅ (arrows, dots, slide anim) |

## Tier 3 — overlays & portals (floating, focus, dismissal)
Need `egui::Area`/`Window`/popup + open-state management. Harder: positioning,
click-outside dismiss, focus.

| # | Component       | Notes |
|---|-----------------|-------|
| 29| Tooltip         | hover popup — ✅ |
| 30| Popover         | click-anchored floating panel — ✅ |
| 31| Dropdown Menu   | menu items, separators, submenus — ✅ |
| 32| Context Menu    | right-click menu — ✅ |
| 33| Select          | combobox with listbox popup — ✅ |
| 34| Combobox        | searchable select — ✅ (outline trigger → Popover hosting a search Input over a check-marked, case-insensitive filtered list; auto-focus + reset on open; `empty_text` for no-match; module `combobox`) |
| 35| Dialog          | modal + backdrop — ✅ |
| 36| Alert Dialog    | modal confirm — ✅ |
| 37| Sheet           | edge-slide drawer — ✅ |
| 38| Drawer          | bottom sheet — ✅ |
| 39| Hover Card      | rich hover popover — ✅ (popover-surface card on hover-delay; stays open while pointer is over the card via card_hovered feedback + close grace period; flips above/below; module `hover_card`) |
| 40| Menubar         | app menu bar — ✅ (horizontal trigger row, each hangs a `DropdownMenu`; click-to-open + hover-to-switch between siblings; open trigger keyed in egui memory; reuses the `DropdownMenu` surface; module `menubar`) |
| 41| Command         | command palette — ✅ (search header over grouped, keyboard-navigable items; ↑/↓ wrap+scroll, Enter/click activate, real `Ctrl`-chord accelerators, icon + shortcut chips; module `command`) |
| 42| Navigation Menu | nav with flyout panels — ✅ (horizontal trigger bar; hover-driven flyout content panel per menu via a caller closure `panel(idx, ui)`; rest-to-open / instant-switch-while-open / grace-close state machine + panel-hover keep-open; plain `NavItem::link`s return clicks; module `navigation_menu`) |

## Tier 4 — stateful systems (managers, animation, data)
Cross-cutting: own a store, animate, or coordinate many instances.

| # | Component    | Notes |
|---|--------------|-------|
| 43| Toast/Sonner | ✅ Sonner-faithful: ctx-data queue, Toast::send + Toaster, collapsed peek-stack that expands on hover, timers pause while hovered, scale-toward-corner, variant icons, six positions, per-position stacks (module `sonner`) |
| 44| Calendar     | month grid — ✅ (dependency-free Date, prev/next, today ring, selection) |
| 45| Date Picker  | calendar in a popover — ✅ (outline trigger + Calendar in Popover, CloseOnClickOutside, closes on pick; `.with_input()` variant = typeable field + parser + icon-button; module `date_picker`) |
|45b| Time Picker  | typeable HH:MM:SS field with clock glyph — ✅ (`Time` struct + parser, normalises on commit; module `time_picker`) |
| 46| ~~Form~~      | NOT a current shadcn component — the old react-hook-form `<Form>` (`FormField`/`FormItem`/`FormMessage`) was deprecated and replaced by **Field** ✅. No standalone Form on the registry; treat as done via `field`. |
| 47| Chart        | line/bar/area — ✅ (hand-painted, no egui_plot dep; grouped bars / poly-lines / filled areas over x-axis category `labels` + named `Series` on a `--chart-N` palette; muted cartesian grid w/ y-ticks (nice-ceiling) + x-labels; hover guide + floating tooltip card listing each series w/ swatch; optional legend; module `chart`) |
| 48| Sidebar      | collapsible app shell nav — ✅ (`Sidebar` shell: animates expanded rail ↔ slim icon rail w/ `animate_bool_with_time`; pinned header + footer bracket a scrollable middle; built-in `panel-left` trigger + click-to-toggle edge `rail`; collapse state persists in egui memory by `id_salt`, so an external `SidebarTrigger` anywhere drives it; section closures receive `collapsed`; module `sidebar`. Plus `sidebar_menu` nav-group rows ✅) |
| 49| Input OTP    | segmented one-time-code field — ✅ (N single-char cells bound to one `&mut String`; type to auto-advance, Backspace/Delete edit, arrows/click move caret, blinking caret + focus ring on the active cell; `group(n)` splits with a separator; digits or `alphanumeric`; per-cell gaps; module `input_otp`) |
| 50| Sonner       | ✅ same module as 43 (`sonner`) |

## Newer shadcn additions (post-original-plan)
| Component | Notes |
|-----------|-------|
| Spinner   | indeterminate loader — ✅ (painted arc, animate-spin) |
| Empty     | centered empty-state placeholder — ✅ (media/title/desc/content) |
| Field     | labelled form-field wrapper — ✅ (label + control + desc/error) |
| Item      | content row (icon + text + action) — ✅ |
| Grid      | layout helper — ✅ |
| Input Group | input with leading/trailing addons — ✅ |
| Button Group | joined button segments — ✅ |
| Typography | prose scale h1…muted + blockquote/list/inline-code — ✅ (`Variant` enum, `fonts::bold` helper, `List` builder; module `typography`) |
| Native Select | bordered native-style picker — ✅ (outlined `h-9` trigger + chevron, menu popup; module `native_select`) |

## Chat / AI surfaces (shadcn's radix `message` family)
| Component | Notes |
|-----------|-------|
| Marker    | inline status / bordered row / labelled separator — ✅ (`Variant` default/border/separator, leading icon, animated `shimmer` for streaming text; module `marker`) |
| Message   | conversation row — ✅ (`Side` start/end alignment, bubble surface from `muted`/`primary`, header/footer slots with footer following the side, avatar anchored to the bubble bottom & clear of the footer; `MessageGroup` stacks consecutive senders; module `message`) |
| Message Scroller | streaming chat transcript scroller — ✅ (`auto_scroll` follows the live edge only while at it via egui `stick_to_bottom`; `anchor` settles new turns near the top with `previous_item_peek`; `default_position` start/end/last-anchor; place-keeping on prepend; floating jump-to-latest button; module `message_scroller`) |

## Remaining for full coverage (partials only)
**Missing (real, from the registry):** none.
**Bubble (New)** — ✅ extracted standalone (module `bubble`): seven `Variant`s
(Default/Secondary/Muted/Tinted/Outline/Ghost/Destructive), start/end `Align`,
overlapping `reactions` with `Side` + align, 80%-width sizing (ghost = full
width, unframed), `BubbleGroup` for consecutive senders. `Message` keeps the
row-level avatar/name/footer chrome; `Bubble` is just the surface.
**Not a component:** Form — deprecated by shadcn, superseded by Field ✅.
**Done (chat):** Attachment ✅ (radix-rhea file chip: card-surface chip with a
media tile — a `Kind::from_name` lucide glyph (image/document/audio/video/
archive/file) or an image preview — a name over a `TYPE · SIZE` description,
optional remove × action; upload `State` {Idle,Uploading,Processing,Error,Done}
with a shimmering title while busy + destructive treatment on error; `Size`
{Default,Sm,Xs} and `Orientation` {Horizontal,Vertical}; returns
`AttachmentResponse { response, removed }`; module `attachment`).
**Data Table** ✅ — `DataTable` over [`Table`]'s presentation: a global text
filter, click-to-sort columns (cycling asc → desc → unsorted, numeric or
lexicographic), and pagination via `Pagination`. Sort/filter/page state persists
in egui memory keyed by `id_salt`, so it's built fresh each frame from the data
(`DataColumn` builder: `align`/`sizing`/`unsortable`/`numeric`); module
`data_table`.
**Sidebar** ✅ — collapsible `collapsible="icon"` app-shell (`Sidebar` +
`SidebarTrigger`, module `sidebar`); see registry row 48.
**Partial:** Direction/RTL (none).
Suggested order: Direction/RTL.

**Done since last refresh:** Typography ✅, Native Select ✅, Combobox ✅,
Hover Card ✅, Command ✅ (palette engine: grouped keyboard-navigable item list
over a search header; ↑↓ wrap + scroll-into-view, Enter/click activate, icon +
shortcut Kbd chips, keyword matching; module `command` — the same engine shadcn
reuses inside Combobox/Dialog), Menubar ✅ (app menu bar: click-to-open +
hover-to-switch row of `DropdownMenu`s; module `menubar`), Navigation Menu ✅
(site-header nav with hover-driven flyout content panels; module
`navigation_menu`), Input OTP ✅ (segmented one-time-code field; module
`input_otp`), Attachment ✅ (chat file chip with extension-derived icon, size,
remove button; module `attachment`), Chart ✅ (hand-painted bar/line/area over
named series w/ grid, legend, hover tooltip; module `chart`).

---

## Build protocol
1. Implement the component as a `Widget` in `src/components/<name>.rs`.
2. Variants/sizes as enums; defaults resolved from `ui.style()`.
3. Re-export from `lib.rs`.
4. Add to the eframe demo gallery (`examples/gallery.rs`).
5. `cargo clippy` clean before moving on.

## Starting now: **#1 Button**
The vocabulary-setter — variant + size enums here become the template every other
component copies.
