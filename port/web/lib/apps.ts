// Marketing/landing content for each app in the trial catalog (names match `CATALOG` in crates/odoo-app/src/trial.rs).
export type AppInfo = { name: string; slug: string; category: string; community: boolean; tagline: string; features: string[]; attributes: [string, string][] }

// name | slug | category | community(1)/enterprise(0) | tagline | feature;feature;feature | attribute=value;...
const RAW = `
Website|website|Website|1|Build a fast, editable company website.|Drag-and-drop page builder;Multi-language and SEO tools;Forms that feed your CRM|Replaces=Web agency CMS;Works with=eCommerce, Blog, Events
eCommerce|ecommerce|Website|1|Sell online with products, carts and payments.|Catalog with variants and pricing rules;Checkout with payment and delivery options;Orders flow into Sales and Inventory|Replaces=Standalone web shop;Works with=Sales, Inventory, Invoicing
Blog|blog|Website|1|Publish articles and grow your audience.|Rich-text posts with tags;Author pages and comments;Social sharing and SEO metadata|Replaces=Separate blog platform;Works with=Website, Email Marketing
Forum|forum|Website|1|Let customers and members help each other.|Questions, answers and voting;Karma and moderation;Searchable knowledge base|Replaces=Community forum software;Works with=Website
eLearning|elearning|Website|1|Publish courses and track learner progress.|Courses with videos, documents and quizzes;Progress tracking and certifications;Free or paid access|Replaces=Course platform;Works with=Website, eCommerce
Events|events|Website|1|Plan events and sell tickets.|Ticket types and registration forms;Agenda, tracks and speakers;Attendee check-in|Replaces=Ticketing tools;Works with=Website, Email Marketing
CRM|crm|Sales|1|Track leads and close more deals.|Visual pipeline with stages;Lead scoring and activities;Quotations straight from an opportunity|Replaces=Spreadsheet pipelines;Works with=Sales, Email Marketing
Sales|sales|Sales|1|Quote, confirm and invoice orders.|Quotation templates and pricelists;Online approval and signature;Order-to-invoice automation|Replaces=Manual quotes;Works with=CRM, Inventory, Invoicing
Point of Sale|point-of-sale|Sales|1|A fast retail till that works offline.|Touch-friendly register;Barcode, loyalty and discounts;Stock and accounting updated in real time|Replaces=Standalone POS;Works with=Inventory, Accounting
Restaurant|restaurant|Sales|1|Floor plans, tables and kitchen orders.|Table management and split bills;Kitchen and bar order tickets;Tips and course ordering|Replaces=Restaurant POS;Works with=Point of Sale, Inventory
Subscriptions|subscriptions|Sales|0|Recurring revenue with automatic renewals.|Recurring plans and renewals;Upsell and churn tracking;Automatic invoicing|Replaces=Billing spreadsheets;Works with=Sales, Invoicing
Rental|rental|Sales|0|Rent out equipment with availability tracking.|Rental calendar and pricing by period;Pickup and return tracking;Late fees|Replaces=Booking sheets;Works with=Sales, Inventory
Invoicing|invoicing|Finance|1|Send professional invoices and get paid.|Invoices and credit notes;Payment terms and follow-ups;PDF and email delivery|Replaces=Word or Excel invoices;Works with=Sales, Accounting
Accounting|accounting|Finance|1|Full double-entry books with bank reconciliation.|Chart of accounts and journals;Bank statement reconciliation;Taxes and financial reports|Replaces=Separate accounting package;Works with=Invoicing, Purchase
Expenses|expenses|Finance|1|Collect and reimburse employee expenses.|Submit expenses from receipts;Manager approval flow;Reimbursement and re-invoicing|Replaces=Paper expense forms;Works with=Employees, Accounting
Sign|sign|Finance|0|Collect legally binding e-signatures.|Reusable document templates;Multi-party signing order;Audit trail|Replaces=Standalone e-sign tool;Works with=Sales, Employees
Equity|equity|Finance|0|Manage your cap table and share transactions.|Shareholder registry;Share transfers and rounds;Ownership reports|Replaces=Cap-table spreadsheets;Works with=Accounting
ESG|esg|Finance|0|Measure and report sustainability data.|Emissions tracking;Reporting frameworks;Targets and trends|Replaces=Manual ESG reports;Works with=Accounting
Project|project|Services|1|Plan work with tasks, stages and deadlines.|Kanban, list and Gantt-style planning;Task assignment and sub-tasks;Customer ratings and portal sharing|Replaces=Task trackers;Works with=Timesheets, Sales
Timesheets|timesheets|Services|1|Track time on projects and tasks.|Timers and weekly grids;Billable vs non-billable time;Invoice time to customers|Replaces=Time-tracking sheets;Works with=Project, Invoicing
Field Service|field-service|Services|0|Dispatch technicians and track interventions.|Map and schedule view;Worksheets and signatures on site;Parts consumed and invoiced|Replaces=Dispatch spreadsheets;Works with=Project, Inventory
Helpdesk|helpdesk|Services|0|Resolve customer tickets on time.|Ticket pipeline and SLAs;Email and web intake;Satisfaction ratings|Replaces=Shared inbox;Works with=CRM, Project
Appointments|appointments|Services|0|Let customers book time with you online.|Online booking pages;Calendar sync;Reminders|Replaces=Booking tools;Works with=Website, CRM
Planning|planning|Services|0|Schedule shifts and resources.|Gantt scheduling;Open shifts;Conflict detection|Replaces=Rota spreadsheets;Works with=Employees, Project
Documents|documents|Productivity|0|A shared, searchable document workspace.|Folders and tags;Workflow actions per document;Sharing with expiry|Replaces=Shared drives;Works with=Accounting, Sign
Approvals|approvals|Productivity|0|Request and approve anything.|Custom approval types;Multi-level approvers;Status tracking|Replaces=Email approvals;Works with=Expenses, Purchase
Knowledge|knowledge|Productivity|0|Company wiki and internal documentation.|Nested pages with templates;Permissions per page;Embed views and files|Replaces=Wiki tools;Works with=Project
Inventory|inventory|Supply Chain|1|Control stock across warehouses.|Multi-warehouse and locations;Lots, serial numbers and barcodes;Replenishment rules|Replaces=Stock spreadsheets;Works with=Sales, Purchase, Manufacturing
Manufacturing|manufacturing|Supply Chain|1|Plan production with bills of materials.|Bills of materials and work orders;Shop-floor tablet view;Costing and traceability|Replaces=Production sheets;Works with=Inventory, Purchase
Purchase|purchase|Supply Chain|1|Buy from vendors with RFQs and approvals.|Requests for quotation;Vendor bills three-way match;Reordering from stock rules|Replaces=Manual purchase orders;Works with=Inventory, Accounting
Maintenance|maintenance|Supply Chain|1|Keep equipment running with planned upkeep.|Equipment register;Preventive schedules;Corrective requests|Replaces=Maintenance logbooks;Works with=Manufacturing, Inventory
Quality|quality|Supply Chain|0|Quality checks on every operation.|Control points on receipts and production;Quality alerts;Pass/fail measures|Replaces=Paper checklists;Works with=Inventory, Manufacturing
Repair|repair|Supply Chain|1|Manage repair orders end to end.|Repair orders and parts;Quotations and invoicing;Warranty tracking|Replaces=Workshop tickets;Works with=Inventory, Sales
Email Marketing|email-marketing|Marketing|1|Design and send newsletters.|Drag-and-drop templates;Segmented mailing lists;Open and click statistics|Replaces=Newsletter tools;Works with=CRM, Website
SMS Marketing|sms-marketing|Marketing|1|Reach customers with text messages.|Bulk SMS campaigns;Personalised content;Delivery tracking|Replaces=Bulk SMS services;Works with=CRM
Survey|survey|Marketing|1|Collect feedback with surveys and quizzes.|Multiple question types;Scoring and certifications;Live results|Replaces=Survey tools;Works with=Events, eLearning
Social Marketing|social-marketing|Marketing|0|Schedule and track social posts.|Multi-channel posting;Engagement stats;Content calendar|Replaces=Social schedulers;Works with=Website
Employees|employees|Human Resources|1|Your employee directory and org chart.|Employee profiles and contracts;Org chart;Departments and job positions|Replaces=HR spreadsheets;Works with=Time Off, Recruitment
Attendances|attendances|Human Resources|1|Check in and out with kiosk or badge.|Kiosk and badge check-in;Overtime tracking;Attendance reports|Replaces=Punch clocks;Works with=Employees, Timesheets
Recruitment|recruitment|Human Resources|1|Fill positions with a hiring pipeline.|Job posts on your website;Applicant stages;Interview scheduling|Replaces=Applicant spreadsheets;Works with=Employees, Website
Time Off|time-off|Human Resources|1|Leave requests and balances.|Leave types and allocations;Manager approval;Team calendar|Replaces=Leave spreadsheets;Works with=Employees, Planning
Appraisals|appraisals|Human Resources|0|Run performance reviews.|Review cycles;Goals and feedback;360-degree input|Replaces=Review forms;Works with=Employees
Fleet|fleet|Human Resources|1|Track vehicles, contracts and costs.|Vehicle register and drivers;Service and contract reminders;Fuel and cost reports|Replaces=Vehicle logbooks;Works with=Employees, Maintenance
Payroll|payroll|Human Resources|0|Pay employees accurately and on time.|Salary rules and payslips;Contracts and benefits;Accounting entries|Replaces=Payroll outsourcing;Works with=Employees, Time Off
Studio|studio|Customizations|0|Customise apps without code.|Add fields and views;Automated actions;Custom reports|Replaces=Custom development;Works with=All apps
`
export const APPS: AppInfo[] = RAW.trim().split('\n').map((l) => {
  const [name, slug, category, c, tagline, feats, attrs] = l.split('|')
  return { name, slug, category, community: c === '1', tagline, features: feats.split(';'), attributes: attrs.split(';').map((a) => a.split('=') as [string, string]) }
})
export const appBySlug = (s: string) => APPS.find((a) => a.slug === s)
export const appByName = (n: string) => APPS.find((a) => a.name === n)
