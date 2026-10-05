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

## Languages
UI strings: 11 locales (en, es, fr, de, pt, it, ru, hi, ar [RTL], zh, ja) in `web/lib/i18n`. Data: `tools/extract_i18n.py` builds
`i18n/<lang>.json` from Odoo's `.po` files (~18k field labels, 3k selection values, 790 menus, 4.5k view strings per language); the server
applies them to `fields_get`, `menus` and `get_view` per request language. Gaps: translated record *data* (product names etc.), reports, emails.

## AI assistant
Drawer (Ctrl/⌘+/). **Default provider: Poolside AI** (`https://inference.poolside.ai/v1`, model `poolside/laguna-s-2.1` — ids must be fully qualified).
Providers: Poolside, Ollama, LM Studio, vLLM, Anthropic, OpenAI, Gemini, Groq, xAI, Mistral, DeepSeek, Cerebras, Together,
Fireworks, OpenRouter, custom OpenAI-compatible. Tools (read-only): list_models, describe_model, search_read, count, read_group; they run
through `dispatch` with the caller's session, so ACL and record rules apply. Writes are only *proposed* and need an explicit Approve click.
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

## Free-trial signup (`/trial`)
Public page modelled on odoo.com/trial: *Choose your Apps* (45 cards in Odoo's categories; the 28 that exist in this edition are selectable,
Enterprise-only ones are shown greyed), then a signup form (name, work email, phone, company, country, password, terms), then provisioning and
auto-login. Each signup gets **its own SQLite database** under `trials/` with only the chosen apps (+ dependencies) installed, and the visitor
is that database's administrator; the main database is never touched and a trial session token is rejected by it.
Server flags: `--trials true|false` (default true), `--trials-dir trials`, `--trials-max 25`, `--trials-days 15`.
Public RPCs: `trial_catalog`, `trial_create`; every other request routes to a trial via `db`.
Controls: server-side validation of every field and app, one trial per email, capacity cap, 10 signups/minute, trials expire after `--trials-days`,
database names are validated (`trial-<16 hex>`, no path escape), trial databases never use the operator's `ODOO_AI_API_KEY`.
A localized welcome email is sent through the operator's configured email provider (best effort; signup never fails because of it).
**Not included (needed before exposing this publicly):** email *verification* (a confirmation link before activation), CAPTCHA or per-IP rate limiting
(the limit is global), real terms/privacy pages (the checkbox has no link), tenant resource quotas beyond the count cap, and backups of trial data.

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
- RPC: `portal_login|me|logout|update|change_password|open|delete|new_trial`. Portal sessions are separate from tenant sessions (12h TTL).
- Ownership: Open/Delete only work on databases the account owns. Open mints a tenant admin session for the owner.
- Limits: max 3 databases per account; 5 failed logins / 5 min throttle; no account enumeration (unknown email takes the same path and error as a wrong password).
- Account outlives expired trials (purge removes only the database).
- Known gaps: no email-verification link or password reset, no CAPTCHA, portal sessions are in-memory (lost on server restart).

## Point of Sale terminal (`/pos`)

Full-screen register modeled on odoo.com/app/point-of-sale-shop. Home → "Point of Sale" opens the register list (open/continue, Configure → the real `pos.config` form, plus shortcuts to sessions, orders, payment methods, categories, products, pricelists).

- **Selling:** category filter + search, barcode scanner (fast keystrokes + Enter, incl. scale-label EAN `21…`), numpad with Qty / Disc / Price modes, per-line notes, several parked orders in tabs.
- **Pricing:** taxes (percent / fixed / division, price-included), manual discounts (`manual_discount`), price lock (`restrict_price_control`), pricelists with category/product/variant rules and quantity breaks, optional cash rounding.
- **Payment:** multiple/split payments, cash change (only cash can give change), Customer Account (requires a customer), quick-cash buttons, invoice option (`account.move`, needs a customer), receipt with header/footer + print.
- **Offline:** if the server is unreachable the order is queued in `localStorage` and synced later; the server dedupes by order `uuid`, so retries never double-sell.
- **Back office:** order history with refunds (mirror order, per-line remaining quantity, same payment method), session summary / close with counted cash and difference (`set_maximum_difference` honored). First open seeds Cash/Card/Customer Account methods and enables products for POS.
- Server actions: `pos.config.open_ui`, `pos.order.create_from_ui|refund`, `pos.session.summary|close_session`; totals are always recomputed server-side.
- **Not built:** restaurant floors/kitchen/preparation displays, self-order kiosk, loyalty/coupons/gift cards/eWallet, cashier badge/PIN (pos_hr), RFID, hardware proxy/receipt-printer/cash-drawer/scale drivers, payment-terminal integrations (Adyen/Stripe… are configurable but not driven), customer-facing display, stock moves and accounting entries at session close, eCommerce/click-and-collect, multi-currency, non-English POS strings (English fallback).
