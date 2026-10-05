// Industry landing-page content. Each industry preselects the apps it needs in the free trial (`/trial?industry=<slug>`).
// The industry list follows odoo.com/all-industries; features shown on a landing page come from the apps it bundles.
import { APPS, appByName, type AppInfo } from './apps'

export type CategoryProfile = { name: string; summary: string; workflow: string[]; strengths: string[] }
export type Industry = { slug: string; name: string; category: string; audience: string; apps: string[] }

export const CATEGORIES: CategoryProfile[] = [
  { name: 'Business Services', summary: 'Run client work from first contact to final invoice.', workflow: ['Capture the lead', 'Quote and agree scope', 'Deliver and track time', 'Invoice and collect'], strengths: ['Billable time tied to invoices', 'One client record across sales, projects and billing', 'Profitability per project or client'] },
  { name: 'Culture & Arts', summary: 'Sell tickets, works and memberships while looking after your audience.', workflow: ['Promote the programme', 'Sell tickets or items', 'Welcome visitors', 'Follow up with your audience'], strengths: ['Ticketing and point of sale in one place', 'Audience mailing lists', 'Stock for shop items'] },
  { name: 'Education & Training', summary: 'Enrol learners, schedule sessions and bill for them.', workflow: ['Publish courses or sessions', 'Enrol and take payment', 'Teach and track progress', 'Collect feedback'], strengths: ['Online registration', 'Progress and satisfaction tracking', 'Automatic invoicing'] },
  { name: 'Events, Community & Nonprofits', summary: 'Organise people, events and contributions with less admin.', workflow: ['Recruit members or attendees', 'Run events and activities', 'Collect fees or donations', 'Report to stakeholders'], strengths: ['Registrations and ticketing', 'Member and donor records', 'Simple bookkeeping'] },
  { name: 'Food & Beverage', summary: 'Keep the counter, kitchen and stockroom in step.', workflow: ['Plan purchases', 'Prepare or produce', 'Serve and sell', 'Reconcile stock and takings'], strengths: ['Fast till with table or counter service', 'Recipes as bills of materials', 'Real-time stock and costs'] },
  { name: 'Health, Wellness & Personal Care', summary: 'Serve clients, sell products and keep the team scheduled.', workflow: ['Welcome the client', 'Deliver the service', 'Sell products at the till', 'Bring them back'], strengths: ['Client records', 'Service and retail sales at one till', 'Staff management'] },
  { name: 'Hospitality, Tourism & Leisure', summary: 'Fill capacity, serve guests and maintain the venue.', workflow: ['Attract bookings', 'Welcome guests', 'Serve and sell on site', 'Maintain and review'], strengths: ['Online presence that takes bookings', 'On-site sales for food, drink and extras', 'Maintenance of facilities'] },
  { name: 'Manufacturing & Supply Chain', summary: 'Plan, produce, stock and ship with full traceability.', workflow: ['Quote and confirm orders', 'Buy materials', 'Produce or pick', 'Ship and invoice'], strengths: ['Bills of materials and work orders', 'Multi-warehouse inventory', 'Purchase to payment in one flow'] },
  { name: 'Real Estate, Construction & Maintenance', summary: 'Win jobs, run projects and bill by milestone.', workflow: ['Win the project', 'Plan and schedule', 'Buy materials and track time', 'Invoice by milestone'], strengths: ['Projects with tasks and budgets', 'Quotes that become invoices', 'Equipment and site maintenance'] },
  { name: 'Retail & eCommerce', summary: 'Sell in store and online from one stock.', workflow: ['Source products', 'Sell in store or online', 'Fulfil and deliver', 'Re-engage customers'], strengths: ['One catalog for shop and web', 'Barcode-driven stock control', 'Email campaigns to customers'] },
  { name: 'Trades & Home Services', summary: 'Quote jobs, schedule work and invoice on completion.', workflow: ['Receive the request', 'Quote the job', 'Do the work and log time', 'Invoice on site'], strengths: ['Quick quotes', 'Time and materials on each job', 'Fast invoicing'] },
]

// category | slug | name | audience | apps
const RAW = `
Business Services|accounting-firm|Accounting Firm|For accountants, fiduciaries, auditors, and financial advisors|Accounting,Invoicing,Project,Timesheets,CRM,Expenses
Business Services|billboard-rental|Billboard Rental|For advertising companies, media agencies, and outdoor space owners|CRM,Sales,Invoicing,Project,Maintenance,Website
Business Services|audit-certification|Audit & Certification|For auditors, certification bodies, and compliance specialists|Project,Timesheets,CRM,Invoicing,Survey,Employees
Business Services|environmental-agency|Environmental Agency|For consultants, sustainability firms, and inspection services|Project,Timesheets,CRM,Sales,Invoicing,Survey
Business Services|talent-acquisition|Talent Acquisition|For recruiters, staffing agencies, and HR consultancies|Recruitment,CRM,Website,Employees,Email Marketing,Invoicing
Business Services|law-firm|Law Firm|For lawyers, legal advisors, and notaries|Project,Timesheets,CRM,Invoicing,Accounting,Expenses
Business Services|it-hardware-support|IT Hardware & Support|For IT resellers, support providers, and hardware suppliers|Sales,Inventory,Purchase,Project,Timesheets,CRM,Invoicing
Business Services|marketing-agency|Marketing Agency|For agencies, consultants, and branding professionals|CRM,Sales,Project,Timesheets,Email Marketing,Website,Invoicing
Business Services|odoo-partner|Odoo Partner|For Odoo integrators, resellers, and consultants|CRM,Sales,Project,Timesheets,Invoicing,Website
Business Services|software-reseller|Software Reseller|For software distributors, integrators, and resellers|CRM,Sales,Invoicing,Purchase,Website,Email Marketing
Culture & Arts|arts-and-crafts|Arts & Crafts Store|For art dealers, exhibition spaces, and cultural institutions|Point of Sale,eCommerce,Inventory,Website,Email Marketing,Accounting
Culture & Arts|concert-halls|Concert Halls|Music venues and performing arts spaces|Events,Website,Point of Sale,CRM,Email Marketing,Accounting
Culture & Arts|gallery|Gallery|For art dealers, exhibition spaces, and cultural institutions|Sales,eCommerce,Inventory,Events,CRM,Website
Culture & Arts|library|Library|For public libraries, academic collections, and private archives|Inventory,Website,Events,Employees,Survey,Accounting
Culture & Arts|museum|Museum|For museums, heritage sites, and cultural institutions|Events,Point of Sale,Website,Employees,Survey,Accounting
Culture & Arts|photography|Photography|For photographers, studios, and creative agencies|CRM,Sales,Project,Timesheets,Invoicing,Website
Culture & Arts|tattoo-shop|Tattoo Shop|For tattoo artists, nail art salons, and body art studios|Point of Sale,CRM,Inventory,Website,Email Marketing,Accounting
Culture & Arts|theater|Theater|For theatres, playhouses, and cultural institutions|Events,Website,Point of Sale,CRM,Email Marketing,Accounting
Education & Training|diy-workshops|DIY Workshops|For maker spaces, hobby workshops, and creative learning centers|Events,Website,Point of Sale,Inventory,CRM,Email Marketing
Education & Training|driving-school|Driving School|For driving academies, instructors, and vehicle training centers|CRM,Sales,Invoicing,Fleet,Maintenance,Employees,Website
Education & Training|elearning-platform|eLearning Platform|For online courses, coaches, and digital academies|eLearning,Website,eCommerce,Email Marketing,Survey,CRM
Education & Training|student-organization|Student Organization|For associations, clubs, and student unions|Events,Website,CRM,Email Marketing,Accounting,Survey
Events & Community|community-care|Community Care|For youth protection centers, refugee reception facilities, and non-profit shelters|Employees,Attendances,Time Off,Project,Inventory,Accounting
Events & Community|coworking|Coworking|For coworking hubs, shared offices, and flexible workspace providers|CRM,Sales,Invoicing,Events,Website,Accounting
Events & Community|event-management|Event Management|For event planners, agencies, and professional organizers|Events,CRM,Sales,Project,Purchase,Invoicing
Events & Community|members-club|Members Club|For private clubs, associations, and community groups|CRM,Invoicing,Events,Website,Email Marketing,Accounting
Events & Community|nonprofit-organization|Nonprofit Organisation|For charities, NGOs, and associations|CRM,Invoicing,Events,Website,Email Marketing,Accounting
Events & Community|public-institution|Public Institution|For town halls, city councils, municipal departments, and local agencies|Project,Employees,Time Off,Purchase,Inventory,Accounting
Events & Community|sports-club|Sports Facilities|For local clubs, training centers, and all businesses that rent out sports venues|CRM,Invoicing,Events,Point of Sale,Website,Accounting
Events & Community|summer-camps|Summer Camps|For camp organizers, activity leaders, and youth programs|Events,Website,CRM,Invoicing,Employees,Survey
Events & Community|team-sports-club|Team Sports Club|For football, basketball, and other sports teams|CRM,Events,eCommerce,Point of Sale,Inventory,Accounting
Events & Community|wedding-planner|Wedding Planner|For wedding planners, event designers, and bridal consultants|CRM,Sales,Project,Purchase,Invoicing,Website
Food & Beverage|bakery|Bakery|For artisan bakers, pastry shops, and bakery chains|Point of Sale,Manufacturing,Inventory,Purchase,eCommerce,Accounting
Food & Beverage|bar-pub|Bar & Pub|For bars, pubs and cocktail lounges|Restaurant,Point of Sale,Inventory,Purchase,Employees,Accounting
Food & Beverage|beverage-distributor|Beverages Distributor|For beverage wholesalers, drink logistics, and distribution companies|Sales,Inventory,Purchase,Fleet,CRM,Accounting
Food & Beverage|candy-shop|Candy Shop|For confectionery stores, online sweet shops, and chocolatiers|Point of Sale,eCommerce,Manufacturing,Inventory,Email Marketing,Accounting
Food & Beverage|catering|Catering|For event caterers, banquet organizers, and private food service providers|CRM,Sales,Manufacturing,Purchase,Inventory,Invoicing
Food & Beverage|fast-food|Fast Food|For quick-service restaurants, takeaways, and franchise chains|Restaurant,Point of Sale,Inventory,Purchase,Employees,Accounting
Food & Beverage|food-trucks|Food Trucks|For mobile kitchens, street food vendors, and catering trucks|Point of Sale,Inventory,Purchase,Fleet,Events,Accounting
Food & Beverage|restaurant|Restaurant|For restaurants and culinary houses|Restaurant,Point of Sale,Inventory,Purchase,Employees,Accounting,Website
Food & Beverage|takeaway-restaurant|Takeaway Restaurant|For restaurants, fast food, and online delivery providers|Restaurant,Point of Sale,eCommerce,Inventory,Purchase,Accounting
Health & Wellness|beauty-parlor|Beauty Parlor|For the entire beauty service sector, from fast-casual nail bars to luxury beauty institutes|Point of Sale,CRM,Inventory,Employees,Website,Accounting
Health & Wellness|eyewear-store|Eyewear Store|For opticians, eyewear retailers, and optical clinics|Point of Sale,Sales,Inventory,Repair,CRM,Accounting
Health & Wellness|fitness-center|Fitness Center|For gyms, wellness clubs, and personal training studios|CRM,Invoicing,Point of Sale,Employees,Attendances,Website
Health & Wellness|hair-salon|Hair Salon|For salons, barbershops, and hairstyling studios|Point of Sale,CRM,Inventory,Employees,Website,Accounting
Health & Wellness|mental-therapy|Mental Therapy|For mental health and wellbeing professionals, including psychologists, therapists, and practitioners|CRM,Invoicing,Accounting,Website,Survey
Health & Wellness|personal-trainer|Personal Trainer|For fitness coaches, health consultants, and wellness instructors|CRM,Invoicing,Website,Email Marketing,Accounting
Health & Wellness|pet-groomer|Pet Groomer|For dog groomers, pet salons, and animal grooming professionals|Point of Sale,CRM,Inventory,Employees,Website,Accounting
Health & Wellness|pharmacy|Pharmacy|For pharmacies, drugstores, and health retailers|Point of Sale,Inventory,Purchase,eCommerce,Employees,Accounting
Health & Wellness|physical-therapy|Physical Therapy|For physical therapy and rehabilitation practices, including kinesiotherapists, physiotherapists and osteopaths|CRM,Invoicing,Accounting,Employees,Website
Health & Wellness|veterinary-clinic|Veterinary Clinic|For vets, animal hospitals, and pet care providers|Point of Sale,Inventory,Purchase,CRM,Employees,Accounting
Health & Wellness|yoga-pilates|Yoga & Pilates Studio|For yoga studios, pilates trainers, and wellness centers|CRM,Invoicing,Point of Sale,Events,Website,Accounting
Hospitality & Leisure|bowling|Bowling Alleys|For bowling alleys, leisure centers, and entertainment venues|Point of Sale,Restaurant,Events,Inventory,Employees,Accounting
Hospitality & Leisure|campsite|Campsite|For camping grounds, glamping accommodations and cabin rentals|Website,Sales,Invoicing,Point of Sale,Maintenance,Accounting
Hospitality & Leisure|escape-rooms|Escape Rooms|For escape room operators, adventure venues, and group entertainment|Website,Events,Point of Sale,CRM,Employees,Accounting
Hospitality & Leisure|guest-house|Guest House|For B&Bs, homestays, and family-run lodgings|Website,Sales,Invoicing,Point of Sale,Maintenance,Accounting
Hospitality & Leisure|guided-tours|Guided Tours|For city tour operators, local guides, and urban sightseeing companies|Events,Website,CRM,Invoicing,Employees,Accounting
Hospitality & Leisure|holiday-house|Holiday House|For vacation rentals, holiday homes, and property hosts|Website,Sales,Invoicing,Maintenance,CRM,Accounting
Hospitality & Leisure|hotel|Hotel|For hotels, resorts, and hospitality chains|Website,Sales,Restaurant,Point of Sale,Employees,Maintenance,Accounting
Hospitality & Leisure|night-clubs|Night Clubs|For clubs, lounges, and nightlife venues|Point of Sale,Events,Inventory,Purchase,Employees,Accounting
Hospitality & Leisure|outdoor-activities|Outdoor Activities|For adventure parks, tour operators, and outdoor organizers|Events,Website,CRM,Invoicing,Fleet,Accounting
Hospitality & Leisure|spa-resort|Spa Resort|For wellness resorts, spa hotels, and luxury retreats|Website,Sales,Point of Sale,Restaurant,Employees,Accounting
Manufacturing & Supply Chain|3pl-logistic-company|Third-party Logistics|For freight forwarders, couriers, and distribution networks|Inventory,Sales,Purchase,Fleet,Invoicing,Accounting
Manufacturing & Supply Chain|agri-equipment-rental|Agri-Equipment Rental|For tractor hire, harvester rentals, and agricultural machinery specialists|Sales,Invoicing,Maintenance,Fleet,Inventory,Accounting
Manufacturing & Supply Chain|carpenter|Carpenter|For woodworking shops, custom furniture makers, and craftsmen|Sales,Manufacturing,Inventory,Purchase,Project,Invoicing
Manufacturing & Supply Chain|corporate-gifts|Corporate Gifts|For gift suppliers, promotional products and merchandising producers|CRM,Sales,Manufacturing,Inventory,Purchase,eCommerce
Manufacturing & Supply Chain|custom-furniture-production|Custom Furniture Production|For bespoke furniture makers, carpentry workshops, and design studios|Sales,Manufacturing,Inventory,Purchase,Project,Accounting
Manufacturing & Supply Chain|custom-industrial-equipment|Custom Industrial Equipment|For Engineering, procurement, and construction (EPC) project, Engineering-to-Order solutions and Turnkey projects|Sales,Project,Manufacturing,Purchase,Inventory,Maintenance,Accounting
Manufacturing & Supply Chain|food-distribution|Food Distribution|For food factories, artisanal producers, and distribution businesses|Sales,Manufacturing,Inventory,Purchase,Fleet,Accounting
Manufacturing & Supply Chain|metal-fabricator|Metal Fabricator|For workshops, industrial fabricators, and metalworking firms|Sales,Manufacturing,Inventory,Purchase,Maintenance,Accounting
Manufacturing & Supply Chain|micro-brewery|Microbrewery|For craft brewers, and local breweries|Manufacturing,Inventory,Purchase,Point of Sale,Sales,Accounting
Manufacturing & Supply Chain|textile-manufacturing|Textile Manufacturing|For textile producers, carpet makers and leather goods producers|Sales,Manufacturing,Inventory,Purchase,Maintenance,Accounting
Manufacturing & Supply Chain|vineyard|Vineyard|For wineries, vineyards and wine producers|Manufacturing,Inventory,Purchase,eCommerce,Point of Sale,Accounting
Real Estate & Construction|architecture-firm|Architecture Firm|For architects, design studios, and construction planners|Project,Timesheets,CRM,Sales,Invoicing,Accounting
Real Estate & Construction|property-owner-association|Property Owner Association|For homeowner associations, property managers, and condominiums|Invoicing,Accounting,Maintenance,Project,Website,Email Marketing
Real Estate & Construction|general-contractor|General Contractor|For construction firms, renovation specialists, and trade contractors|CRM,Sales,Project,Timesheets,Purchase,Inventory,Accounting
Real Estate & Construction|property-developer|Property Developer|For large-scale developers, civil engineers, and construction project managers|Project,Sales,Purchase,Accounting,CRM,Expenses
Real Estate & Construction|hvac-services|HVAC Services|For heating, ventilation, and air conditioning contractors|Sales,Project,Timesheets,Inventory,Maintenance,Invoicing
Real Estate & Construction|property-management|Property Management|For property managers, rental agencies, and building administrators|CRM,Invoicing,Accounting,Maintenance,Website,Project
Real Estate & Construction|interior-design|Interior Design|For interior design studios, space planners, and home decorators|CRM,Sales,Project,Timesheets,Purchase,Invoicing
Real Estate & Construction|machine-tool-rental|Machine & Tools Rental|For heavy machinery rentals, construction equipment providers, and agricultural tool suppliers|Sales,Invoicing,Maintenance,Inventory,Fleet,Accounting
Real Estate & Construction|real-estate-agency|Real Estate Agency|For agents, brokers, and estate professionals|CRM,Website,Sales,Invoicing,Accounting,Email Marketing
Real Estate & Construction|solar-energy|Solar Energy Systems|For solar installers, energy consultants, and green providers|CRM,Sales,Project,Inventory,Purchase,Invoicing
Retail & eCommerce|agriculture-store|Agricultural Store|For farming suppliers, gardening shops, and agricultural distributors|Point of Sale,eCommerce,Inventory,Purchase,Accounting
Retail & eCommerce|automobile|Automobile Spare Parts|For spare parts retailers, car accessories shops, and auto components suppliers|Point of Sale,eCommerce,Inventory,Purchase,Sales,Accounting
Retail & eCommerce|book-store|Bookstore|For book retailers, comic shops, and independent bookstores|Point of Sale,eCommerce,Inventory,Purchase,Events,Email Marketing
Retail & eCommerce|clothing-store|Clothing Store|For fashion retailers, boutiques, and apparel shops|Point of Sale,eCommerce,Inventory,Purchase,Email Marketing,Accounting
Retail & eCommerce|cosmetics-store|Cosmetics Store|For beauty shops, skincare retailers, and perfume boutiques|Point of Sale,eCommerce,Inventory,Purchase,Email Marketing,Accounting
Retail & eCommerce|dropshipping|Dropshipping|For online sellers, ecommerce operators, and fulfillment businesses|eCommerce,Sales,Purchase,Inventory,Email Marketing,Accounting
Retail & eCommerce|electronic-refurbishment|Electronic Refurbishment|For circular economy businesses, used electronics shops, and tech refurbishers|Repair,Point of Sale,eCommerce,Inventory,Purchase,Accounting
Retail & eCommerce|electronics-store|Electronic Store|For electronics retailers, gadget shops, and home appliance sellers|Point of Sale,eCommerce,Inventory,Purchase,Repair,Accounting
Retail & eCommerce|florist|Florist|For flower shops, event florists, and online floral retailers|Point of Sale,eCommerce,Inventory,Purchase,CRM,Accounting
Retail & eCommerce|grocery-store|Grocery Store|For supermarkets, local shops, and small retailers|Point of Sale,Inventory,Purchase,eCommerce,Employees,Accounting
Retail & eCommerce|furniture-store|Furniture Store|For furniture retailers, showrooms, and interior shops|Point of Sale,Sales,eCommerce,Inventory,Purchase,Accounting
Retail & eCommerce|hardware-store|Hardware Store|For DIY shops, tool retailers, and construction suppliers|Point of Sale,eCommerce,Inventory,Purchase,Sales,Accounting
Retail & eCommerce|thrift-store|Thrift Store|For Thrift Stores, second-hand shop and Resale store|Point of Sale,Inventory,eCommerce,Employees,Accounting
Retail & eCommerce|toy-store|Toy Store|For toy shops, board game retailers, and children's stores|Point of Sale,eCommerce,Inventory,Purchase,Email Marketing,Accounting
Retail & eCommerce|wine-merchant|Wine Shop|For wine shops, distributors, and wine connoisseurs|Point of Sale,eCommerce,Inventory,Purchase,Sales,Accounting
Trades & Home Services|bike-leasing|Bike Leasing|For leasing providers, corporate mobility services, and long-term rental businesses|Sales,Invoicing,Maintenance,Inventory,Fleet,Accounting
Trades & Home Services|bike-shop|Bike Shop|For bicycle retailers, repair workshops, and cycling specialists|Point of Sale,Repair,Inventory,Purchase,eCommerce,Accounting
Trades & Home Services|cleaning-services|Cleaning Services|For cleaning companies and janitorial services|CRM,Sales,Project,Employees,Attendances,Invoicing
Trades & Home Services|electricians|Electrician|For electricians, wiring contractors, and technical service providers|CRM,Sales,Project,Timesheets,Inventory,Invoicing
Trades & Home Services|gardening|Gardening|For landscaping, groundskeeping and outdoor service providers|CRM,Sales,Project,Timesheets,Inventory,Invoicing
Trades & Home Services|handyman|Handyman Services|For repair services, maintenance workers, and general contractors|CRM,Sales,Project,Timesheets,Inventory,Invoicing
Trades & Home Services|surveyor|Surveying & Mapping|For surveyors, mapping firms, and engineering services|CRM,Sales,Project,Timesheets,Invoicing,Accounting
`
const CAT_ALIAS: Record<string, string> = { 'Events & Community': 'Events, Community & Nonprofits', 'Health & Wellness': 'Health, Wellness & Personal Care', 'Hospitality & Leisure': 'Hospitality, Tourism & Leisure', 'Real Estate & Construction': 'Real Estate, Construction & Maintenance' }
export const INDUSTRIES: Industry[] = RAW.trim().split('\n').map((l) => {
  const [c, slug, name, audience, apps] = l.split('|')
  return { slug, name, category: CAT_ALIAS[c] ?? c, audience, apps: apps.split(',') }
})

export const industryBySlug = (s: string) => INDUSTRIES.find((i) => i.slug === s)
export const categoryProfile = (name: string) => CATEGORIES.find((c) => c.name === name)
export const industriesByCategory = (): [string, Industry[]][] => CATEGORIES.map((c) => [c.name, INDUSTRIES.filter((i) => i.category === c.name)] as [string, Industry[]]).filter(([, l]) => l.length)
export const industryApps = (i: Industry): AppInfo[] => i.apps.map((n) => appByName(n)).filter((a): a is AppInfo => !!a)
/** Apps the free trial can install for this industry (Enterprise-only ones are skipped). */
export const trialApps = (i: Industry): string[] => industryApps(i).filter((a) => a.community).map((a) => a.name)
/** Industries that use an app, for the app landing page. */
export const industriesUsing = (appName: string): Industry[] => INDUSTRIES.filter((i) => i.apps.includes(appName))
export const searchIndustries = (q: string): Industry[] => { const s = q.trim().toLowerCase(); return s ? INDUSTRIES.filter((i) => `${i.name} ${i.audience}`.toLowerCase().includes(s)) : INDUSTRIES }
export { APPS }
