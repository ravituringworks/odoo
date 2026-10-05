# Apps, import, settings, themes, languages, AI assistant

## Install apps at runtime
`Apps & Install` (sidebar) lists all modules from the extracted manifest (656 incl. imported ones). `install_module(name)` loads the
dependency closure, diffs the schema (`ddl::upgrade_plan`: new tables, `ALTER TABLE .. ADD COLUMN`, indexes, FKs), seeds only the new
modules' data, reloads ACL / record rules / menus and swaps the runtime without a restart. Installed roots are stored in the `odoo_modules` table.
Admin only. Uninstall is not implemented. Storefront pages (QWeb/controllers of `website`, `website_sale`) are not rendered: the back-office
(orders, products, categories, payment providers, delivery) installs and works.

## Import Python addons
`python3 tools/import_addons.py /path/to/addons` registers the path (`addons_paths.txt`), re-extracts models/fields/views/menus/ACL/rules/
data, and writes `imports/<module>.md` listing the Python methods (computes, onchanges, buttons) that still need Rust rules.
Method bodies are not translated automatically. Example: `examples/addons/my_library`.

## Settings (one screen)
Appearance (430 VibeCody themes, flash-free boot), Language, AI, Account (password), Administration (system info + Odoo's Users & Companies,
Translations, Technical menus). Odoo's own "Settings"/"Apps" roots are merged into these; every `res.config.settings` menu opens `/settings`.

## Design system
`web/app/vibe-tokens.css` and `web/lib/vibe/themes.ts` are imported from VibeCody (MIT, same author). Odoo-RS styles alias the tokens, so a theme change restyles everything.

**Iconography.** No emoji or pictographic glyphs anywhere in the UI: every icon is a thin line SVG (`web/components/Icon.tsx`, paths in `web/lib/icons.ts`;
24px grid, round caps, stroke from `--icon-stroke` = 1.5, sizes from `--icon-xs … --icon-xl`, colour from `currentColor`). `lib/icons.ts` also maps every trial-catalog
app, industry segment and portal site to an icon. The icon paths are hand-drawn in Lucide's style (no icon package is installed). `web/lib/icons.test.ts` fails the build
if an emoji/pictograph/arrow glyph reappears in `app/`, `components/` or `lib/` (the ⌘ in a shortcut label and the two POS keypad key values are the only exceptions).
New icons: add the paths to `ICONS` and run the contact-sheet check in the test.

## Multi-company
A session works in one **active company** and a set of **allowed companies** (sessions are in memory: `Session { uid, company_id, allowed }`). A login starts in the user's own company only.
- RPC: `companies` (the user's companies plus the session's current/allowed) and `switch_company [ids]` (first id = active; every id must be one of the user's `company_ids`; an empty or foreign list is refused and changes nothing).
- Record rules: `company_ids`, `user.company_id` and `company_id` in `ir.rule` domains are evaluated against the session (`security::eval_domain_in`); the environment carries `company_id` and `allowed_company_ids` in its context (`RecordRules::domain_ctx`). New records default their `res.company` many2one to the active company.
- The superuser skips ordinary rules but is still scoped by global company rules, which is what makes switching visible for a trial administrator.
- UI: a sidebar switcher (`Shell.tsx`) appears for users with 2+ companies; ticking toggles visibility, clicking a name makes it active.
- Limits: requests with no session token (no-auth/Tauri dev mode) are not company-scoped; `parent_of` is evaluated as a plain match.

## Languages
UI strings: 11 locales (en, es, fr, de, pt, it, ru, hi, ar [RTL], zh, ja) in `web/lib/i18n`. Data: `tools/extract_i18n.py` builds
`i18n/<lang>.json` from Odoo's `.po` files (~18k field labels, 3k selection values, 790 menus, 4.5k view strings per language); the server
applies them to `fields_get`, `menus` and `get_view` per request language. Gaps: translated record *data* (product names etc.), reports, emails.

## AI assistant
Drawer (Ctrl/⌘+/). **Default provider: Poolside AI** (`https://inference.poolside.ai/v1`, model `poolside/laguna-s-2.1` — ids must be fully qualified).
Providers: Poolside, Ollama, LM Studio, vLLM, Anthropic, OpenAI, Gemini, Groq, xAI, Mistral, DeepSeek, Cerebras, Together,
Fireworks, OpenRouter, custom OpenAI-compatible. Tools (read-only): setup_guide, list_models, describe_model, search_read, count, read_group; they run
through `dispatch` with the caller's session, so ACL and record rules apply. Writes are only *proposed* and need an explicit Approve click.
**How-to questions** ("set up an online store for Australia") use `setup_guide` (`crates/odoo-app/src/ai_guides.rs`): guides for ecommerce, pos, sales, inventory, accounting, crm, manufacturing and project return an ordered plan whose steps are
checked against the caller's real data (module installed, company country/currency, products/payment providers/delivery methods exist …), plus per-country notes (AU, NZ, GB, US, CA, DE, FR, IN: currency, tax, `l10n_*` module, shipping/payment hints).
`propose_action` also accepts `kind: "install"` (`values: {module}`); approving it runs `install_module` (admin only) and reloads. Loop guards: identical repeated tool calls are not re-run, `list_models` matches any keyword (with business-word synonyms), and when the 6-round tool budget is used up the model gets one tool-less call to answer from what it gathered (the old "tool-call limit" failure only remains as a last-resort fallback).
Tool output is treated as untrusted; client-supplied system/tool messages are dropped. API keys are stored in `odoo_settings`
(plaintext in the database; env `ODOO_AI_API_KEY` overrides) and are never returned by the API. Small local models can misread data:
review answers before acting.

## Point of Sale payment terminals (configuration)
Selections computed by methods (`selection=lambda self: self._get_payment_terminal_selection()`, `_get_payment_method_type`) are resolved
statically by the extractor, including `super() + [...]` chains and imperative `sel = [..]; sel.append(..)` bodies, and merged per *installed*
module (install `pos_adyen`/`pos_stripe`/`pos_six`… to get their options). 16 of 25 such fields resolve; the rest are runtime-dynamic
(DB-driven languages, chart templates, model lists). Non-stored computes that are ported (e.g. `hide_use_payment_terminal`) are now returned
by `read`, so view conditions like `invisible="hide_use_payment_terminal"` work.
**Not ported:** the POS cashier front end (the OWL app at `/pos/ui`: product grid, cart, payment screen, terminal interaction), session
open/close accounting, and the vendor terminal protocols themselves (Adyen/Stripe/SIX APIs).

### Poolside key from VibeCody
VibeCody keeps provider keys in its machine-bound encrypted store (`~/.vibecli/profile_settings.db`). The key was copied into this app's
settings once, by a throwaway program that opens that store through VibeCody's own `vibe-profile-store` crate and posts the key to
`settings_set` over localhost (never printed or written to disk). To repeat or rotate: `vibecli set-key poolside <key>` in VibeCody, then re-run an
equivalent import, or paste the key in Settings → AI. The key lives in `odoo_settings` inside the app database (plaintext at rest, as documented
above), so the dev database is gitignored; set `ODOO_AI_API_KEY` to keep the key out of the database entirely.

## Industry and app landing pages (`/industries`, `/app`)
Public pages (no login) modelled on odoo.com/all-industries. `/industries/` lists 105 industries in 11 segments (searchable); `/industries/<slug>/` shows the audience, key attributes, the segment's
workflow and strengths, the apps it bundles (each with its top features) and related industries; `/app/` and `/app/<slug>/` do the same for the 45 catalog apps (features, attributes, "popular in"
industries; Enterprise-only apps are marked and have no trial button). Content lives in `web/lib/apps.ts` and `web/lib/industries.ts` (the industry→apps bundles are this port's own choices, not Odoo's);
landing copy is English only. Buttons go to `/trial/?industry=<slug>` or `/trial/?app=<slug>`, which preselects the available apps (`preselect` in `lib/trial.ts`) and says "Configured for …".
`lib/industries.test.ts` checks the app list against the server's trial catalog in `trial.rs`.

## Free-trial signup (`/trial`)
Public page modelled on odoo.com/trial: *Choose your Apps* (45 cards in Odoo's categories; the 28 that exist in this edition are selectable,
Enterprise-only ones are shown greyed), then a signup form (name, work email, phone, company, country, password, terms), then provisioning and
auto-login. Each signup gets **its own SQLite database** under `trials/` with only the chosen apps (+ dependencies) installed, and the visitor
is that database's administrator; the main database is never touched and a trial session token is rejected by it.
Server flags: `--trials true|false` (default true), `--trials-dir trials`, `--trials-max 25`, `--trials-days 15`.
Public RPCs: `trial_catalog`, `trial_create`; every other request routes to a trial via `db`.
Controls: server-side validation of every field and app, one trial per email, capacity cap, 10 signups/minute, trials expire after `--trials-days`,
database names are validated (`trial-<16 hex>` for trials, a restricted `a-z0-9_` name for administrator-created databases; no path escape), trial databases never use the operator's `ODOO_AI_API_KEY`.
A localized welcome email is sent through the operator's configured email provider (best effort; signup never fails because of it).
**Not included (needed before exposing this publicly):** email *verification* (a confirmation link before activation), CAPTCHA or per-IP rate limiting
(the limit is global), real terms/privacy pages (the checkbox has no link), and tenant resource quotas beyond the count cap. (Backups, archiving and per-account app rules are in the admin console below.)

## Admin console: databases and account app rules (`/admin/databases`)
Admin only (uid 1 or `base.group_system` on the **main** database; trial tenants and anonymous callers are refused). Shows the main database, every trial and every
administrator-created ("standard") database with owner, apps, size, days until expiry and backup count; filter by type/status and search.
- **Backup**: a consistent SQLite snapshot (`Store::snapshot`, `VACUUM INTO`) in `<trials dir>/backups/<db>__<yyyymmdd>-<hhmmss>-<hex>.sqlite`; works for the main database and archived ones. List and delete backups per database.
- **Restore** (trials and standard databases): needs the database name typed; takes a safety backup first, swaps the file, ends all sessions on that database, and puts the old file back if the restored one does not open. Backup file names are matched against the database's own list (no path input).
- **Archive / unarchive**: closes the database and refuses all access (portal Open too) while keeping the data; archived trials never expire.
- **Delete**: name typed to confirm; removes the database's backups unless "keep its backups" is ticked.
- **Open**: signs the admin into any database as its administrator (the console session is stashed; "← Back to admin" returns).
- **New (non-trial) database**: `admin_db_create {name?, email?, company, apps}`: permanent, not counted against the trial cap; the owner (optional) must be an existing portal account.
- **Per-account app rules**: each portal account has a list of disallowed catalog apps (`odoo_portal_users.disallowed`). Disallowed apps cannot be provisioned for that account and cannot be installed later into any of its databases (also blocked as a dependency); applied immediately to open databases. A module still needed by an allowed app stays available (e.g. blocking Invoicing but not Accounting). Already-installed apps are not removed.
- Registry: `odoo_trials` gained `kind` (trial|standard) and `status` (active|archived); RPCs are `admin_db_list|backup|backups|backup_delete|restore|archive|delete|open|create` and `admin_account_list|set_apps`.
- Limits: the **main** database can be backed up but not restored/archived/deleted while the server runs; backups are SQLite-only (other engines answer with an error); no scheduled or off-site backups; the portal list does not yet mark archived databases.

## Email (Settings → Email)
Providers: **SendGrid, Mailgun, Postmark, Resend, Brevo** (HTTP APIs) and **SMTP** (Gmail, Office 365, Amazon SES, any server; starttls/ssl/none).
Each provider declares its own settings fields (`email_providers` in `settings_get`), so the UI form is data-driven. Admin only; API keys and the SMTP
password are write-only (reported only as `has_api_key` / `has_smtp_password`; env `ODOO_EMAIL_API_KEY` / `ODOO_SMTP_PASSWORD` override them).
Sending paths: Settings → "Send test email"; admin RPC `send_mail`; the `mail.mail` queue (`send` on records, `process_email_queue`) with sent/exception
state and failure reason; trial welcome message. The From address is always the configured one (no spoofing); recipients are validated (no CR/LF, ≤50,
Bcc only in the SMTP envelope / provider field, never as a header); body ≤1 MB; 30 sends/minute per instance; provider error text is returned with the key redacted.
Trial tenants have their own (empty) settings and never use the operator's credentials, so a visitor cannot send mail through the operator's account.
Verified: all five HTTP providers against a local fake server (request shape, auth header, ids, errors), SMTP against a scripted server, and the browser flow
(configure -> save -> test email) against a local SendGrid stand-in. **Not verified against the real services** (no accounts/credentials here).
Not included: attachments, bounce/webhook handling, DKIM/SPF guidance, per-user sender addresses, inbound mail, and the Python `ir.mail_server` models
(this port uses its own settings instead).

## Customer portal (`/my`)

Modeled on odoo.com/my. A trial signup now creates a **portal account** (`odoo_portal_users`) and lands on `/my/?new=<db>`.

- Pages: `/my` (app tiles, contact card, "Your database is ready!" banner), `/my/databases` (list, Open, two-step Delete, create another), `/my/security` (change password), `/my/details` (edit name/company/phone/country).
- RPC: `portal_login|me|logout|update|change_password|open|delete|new_trial` (Open is refused for archived databases; app rules set in the admin console apply to "new database"). Portal sessions are separate from tenant sessions (12h TTL).
- Ownership: Open/Delete only work on databases the account owns. Open mints a tenant admin session for the owner.
- Limits: max 3 databases per account; 5 failed logins / 5 min throttle; no account enumeration (unknown email takes the same path and error as a wrong password).
- Account outlives expired trials (purge removes only the database).
- Known gaps: no email-verification link or password reset, no CAPTCHA, portal sessions are in-memory (lost on server restart).

## Point of Sale (`/pos`)

Full-screen register modeled on odoo.com/app/point-of-sale-shop. Home → "Point of Sale" opens the register list (open/continue, Configure → the real `pos.config` form, kitchen display, QR cards, reports, plus shortcuts to sessions, orders, payment methods, categories, products, pricelists).

**Selling** — category filter + search; barcode scanner (fast keystrokes + Enter, scale-label EAN `21…`); numpad with Qty / Disc / Price modes; per-line notes; parked orders in tabs; taxes (percent / fixed / division, price-included); manual discounts (`manual_discount`); price lock (`restrict_price_control`); pricelists with category/product/variant rules and quantity breaks; optional cash rounding; split payments, change on cash only, Customer Account, invoice (`account.move`); receipt with header/footer.

**Loyalty (pos_loyalty)** — Odoo's own `loyalty.*` models: points per order/money/unit with thresholds and product/category rules, loyalty cards, coupons, promo codes, buy-X-get-Y / free product, gift cards and eWallet (per-point redemption), next-order coupons. Rewards are recomputed server-side from the program (tax-split per line group); the terminal only previews. A refund reverses the order's points. Scanning a card code (keyboard-wedge RFID/barcode) opens the rewards dialog.

**Cashiers (pos_hr)** — lock screen, PIN or badge scan; *advanced* employees refund, discount, reprice and close the register, *basic* ones only sell (enforced on the server).

**Restaurant (pos_restaurant)** — floor plan with tables, guests, table orders saved as shared drafts (any terminal / refresh sees them), send to kitchen (delta tickets, cancellations), bill, split, transfer table, takeaway, tips (tip product), pay-in-place of a draft.
**Preparation display** — `addons/pos_preparation_display` (a real Odoo-style addon, imported with `tools/import_addons.py`) + `/pos/kitchen/?config=ID[&cat=…]`: live tickets, tap a line to advance new → preparing → ready, bump to serve, late-ticket colours, new-ticket beep.
**Self-order (pos_self_order)** — public `/order/?config=ID&token=…[&table=N]`: QR menu, mobile ordering, kiosk mode (big UI, auto-reset), takeaway, pickup time (click & collect, `ship_later`), live order status (received → preparing → ready). Token-gated; visitors can only pick products/quantities (server prices, no discounts, only POS-enabled products, `kiosk-…` uuids only). `/pos/qr/?config=ID` prints a QR per table.

**Back office** — stock: a delivery (or return) picking is validated per order; accounting: one balanced journal entry per session at close (payments by method, sales, taxes, invoiced settlements, cash difference); order history with refunds; session summary / close with counted cash; sales dashboard `/pos/reports/` (all registers, top products, payment mix, hours/days).

**Hardware** (⚙ in the top bar, settings per browser) — ESC/POS receipts and kitchen/bar tickets (per-printer product categories) to network printers on TCP 9100 through a server bridge that only reaches printers configured on the register, or to USB/serial printers via Web Serial; cash-drawer pulse on cash payments; scales via Web Serial (stable-reading parser) or manual entry; customer-facing display (`/pos/display/?config=ID` — second window live, or another device with `&token=` through the register relay); barcode/RFID readers (keyboard wedge).
**Card terminals** — provider-neutral start/status/cancel; built-in simulator (`pos_terminal_simulator` addon: approves after ~1.5 s, amounts ending .13 declined), Stripe Terminal (server-driven; secret key in Settings → Payment terminals) and Adyen cloud TerminalAPI (key + terminal id on the payment method). A card payment line is accepted only with a one-time server-side approval matching method and amount.

**Offline** — orders queue in `localStorage` when the server is unreachable and sync later; the server dedupes by order `uuid`.

**Added in the limits pass**
- **Card terminals**: Mercado Pago, Viva Wallet, Razorpay (Ezetap), PayTM (AES-CBC signed) and Pine Labs drivers, written from the request shapes in Odoo's own `pos_*` addons, server-side; refunds to the card (Stripe, Adyen reversal, Mercado Pago via API; others by manual confirmation); Settings → Payment terminals has a read-only *Test connection* per method (Stripe reader list, Mercado Pago device, Viva token).
- **Printers**: Epson ePOS-Print XML (`epson_printer_ip`) through a configured-printers-only server bridge, alongside raw ESC/POS and Web Serial.
- **Floor editor**: drag-and-drop tables (snap 10 px), resize, round/square, number/seats, add/delete tables, add/rename floors.
- **Customer display**: slideshow of ad images kept on the display device.
- **Cashier PIN**: 5 wrong PINs lock the employee for 5 minutes (counter shared by all terminals); badge scans are exempt.
- **Lots & serials**: tracked products need serial/lot numbers at the till (known lots suggested, serials unique and not sold twice unless returned); the exact lot quant leaves stock.
- **Combos**: choose one item per part; server prices parent + items, links child lines to their combo.
- **Multi-currency**: pricelists in another currency convert list prices at the latest rate; orders keep their `currency_rate`; reports, summaries and the closing entry convert back to company currency.
- **UBL e-invoice**: Peppol-BIS-shaped UBL 2.1 Invoice/CreditNote for orders with a customer (`pos.order.ubl`, ⬇ UBL on the receipt).
- **Online shop** `/shop/?config=ID&token=…`: catalogue with pictures, cart, customer details, delivery or pickup; becomes a confirmed `sale.order` (stock delivery follows) or a pickup order on the register; confirmation email when email is configured; token-gated, rate-limited (120/h), never edits existing contacts.

**Still not built / limits**: SIX (client-side SDK over the terminal's websocket) and the Odoo IoT Box printer protocol; live verification of Stripe/Adyen/Mercado Pago/Viva/Razorpay/PayTM/Pine Labs — every driver is tested against mock HTTP only, use *Test connection* and a small real payment before relying on one; Viva/Razorpay/PayTM/Pine Labs refunds are manual; country fiscal printers/fiscal black boxes; loyalty points on foreign-currency orders use the order currency; eCommerce is this catalogue + checkout, not `website_sale`'s themes/payment providers/carriers; translations are machine-made and unreviewed by native speakers.
