// Loads the packaged Wegwijs companion in Chromium with a mocked miraCompanion,
// mocked geolocation and recorded API answers, starts a navigation, drives fixes
// along the route, searches restaurants and records every message for the glasses.
const { chromium } = require('playwright');
const fs = require('fs'); const path = require('path');
const htmlPath = process.argv[2], outDir = process.argv[3];
const fx = f => fs.readFileSync(path.join(__dirname, 'fixtures', f));
const valhalla = JSON.parse(fx('valhalla.json'));
// Decode the recorded route once, to build synthetic Overpass streets around it.
function decode(str, prec){ let i = 0, lat = 0, lon = 0; const out = [];
  while (i < str.length) { let b, sh = 0, r = 0; do { b = str.charCodeAt(i++) - 63; r |= (b & 31) << sh; sh += 5; } while (b >= 32); lat += (r & 1) ? ~(r >> 1) : (r >> 1);
    sh = 0; r = 0; do { b = str.charCodeAt(i++) - 63; r |= (b & 31) << sh; sh += 5; } while (b >= 32); lon += (r & 1) ? ~(r >> 1) : (r >> 1); out.push([lat / prec, lon / prec]); } return out; }
const routePts = decode(valhalla.trip.legs[0].shape, 1e6);
function overpassFixture(){
  const els = []; let id = 1;
  const kinds = ['residential', 'footway', 'tertiary', 'residential', 'service'];
  // Parallel offset streets and perpendicular crossings along the route.
  for (let k = 0; k < 2; k++) {
    const off = (k === 0 ? 1 : -1) * 0.0009;
    for (let i = 0; i + 12 < routePts.length; i += 12) {
      const g = routePts.slice(i, i + 13).map(([la, lo]) => ({lat: la + off * 0.6, lon: lo + off}));
      els.push({type: 'way', id: id++, tags: {highway: kinds[i % kinds.length]}, geometry: g});
    }
  }
  for (let i = 6; i < routePts.length; i += 9) {
    const [la, lo] = routePts[i]; const a = (i * 37) % 180; const dx = Math.cos(a * Math.PI / 180), dy = Math.sin(a * Math.PI / 180);
    els.push({type: 'way', id: id++, tags: {highway: i % 3 ? 'residential' : 'secondary'}, geometry: [{lat: la - dy * 0.0025, lon: lo - dx * 0.004}, {lat: la, lon: lo}, {lat: la + dy * 0.0025, lon: lo + dx * 0.004}]});
  }
  return {version: 0.6, elements: els};
}
const TA_PLACES = [
  {id: '101', name: 'Umami Sushi & Grill', rating: '4.5', reviews: '612', cuisine: ['japanese', 'sushi', 'asian'], price: '€€ - €€€', street: 'Korenlei 12', lat: 51.0552, lon: 3.7201},
  {id: '102', name: 'Thai Garden Gent', rating: '4.0', reviews: '288', cuisine: ['thai', 'asian'], price: '€€', street: 'Vlaanderenstraat 44', lat: 51.0498, lon: 3.7288},
  {id: '103', name: 'Brasserie De Graslei', rating: '4.5', reviews: '1340', cuisine: ['belgian', 'european'], price: '€€ - €€€', street: 'Graslei 7', lat: 51.0546, lon: 3.7210},
  {id: '104', name: 'Pho Saigon', rating: '4.5', reviews: '150', cuisine: ['vietnamese', 'asian', 'soups'], price: '€', street: 'Sint-Pietersnieuwstraat 102', lat: 51.0450, lon: 3.7260},
  {id: '105', name: 'Wok Palace', rating: '3.5', reviews: '90', cuisine: ['chinese', 'asian'], price: '€', street: 'Kortrijksesteenweg 10', lat: 51.0400, lon: 3.7150},
  {id: '106', name: 'Kimchi House', rating: '4.5', reviews: '77', cuisine: ['korean', 'asian'], price: '€€', street: 'Nederkouter 5', lat: 51.0480, lon: 3.7210},
];
const TA_SIGHTS = [
  {id: '201', name: 'Gravensteen', rating: '4.5', reviews: '9800', cats: ['Kastelen', 'Historische bezienswaardigheden'], street: 'Sint-Veerleplein 11', lat: 51.0574, lon: 3.7207, rank: '#1 van 210 bezienswaardigheden in Gent', desc: 'Middeleeuwse gravenburcht uit 1180 met wapenmuseum en uitzicht over de stad vanaf de weergang.'},
  {id: '202', name: 'Sint-Baafskathedraal', rating: '4.5', reviews: '7600', cats: ['Kerken en kathedralen'], street: 'Sint-Baafsplein', lat: 51.0530, lon: 3.7270, rank: '#2 van 210 bezienswaardigheden in Gent', desc: 'Gotische kathedraal met het Lam Gods van de gebroeders Van Eyck.'},
  {id: '203', name: 'MSK Museum voor Schone Kunsten', rating: '4.5', reviews: '2100', cats: ['Kunstmusea'], street: 'Fernand Scribedreef 1', lat: 51.0376, lon: 3.7220, rank: '#5 van 210 bezienswaardigheden in Gent', desc: 'Oude en moderne kunst van de middeleeuwen tot de twintigste eeuw in het Citadelpark.'},
  {id: '204', name: 'Belfort van Gent', rating: '4.5', reviews: '4300', cats: ['Torens en uitzichtpunten', 'Historische bezienswaardigheden'], street: 'Sint-Baafsplein', lat: 51.0536, lon: 3.7249, rank: '#4 van 210 bezienswaardigheden in Gent', desc: 'Werelderfgoedtoren van 91 meter met beiaard en draak; lift naar het uitzichtplatform.'},
];
const taSight = p => ({location_id: p.id, name: p.name, web_url: 'https://www.tripadvisor.com/Attraction_Review-' + p.id, address_obj: {street1: p.street, city: 'Gent', address_string: p.street + ', 9000 Gent'}, latitude: String(p.lat), longitude: String(p.lon), rating: p.rating, num_reviews: p.reviews, description: p.desc,
  ranking_data: {ranking_string: p.rank}, subcategory: p.cats.map(c => ({name: c.toLowerCase(), localized_name: c})), groups: [{name: 'Sights & Landmarks', localized_name: 'Bezienswaardigheden en monumenten', categories: p.cats.map(c => ({name: c.toLowerCase(), localized_name: c}))}],
  hours: {periods: [0,1,2,3,4,5,6].map(d => ({open: {day: d, time: '1000'}, close: {day: d, time: '1800'}})), weekday_text: ['maandag: 10:00 - 18:00', 'dinsdag: 10:00 - 18:00', 'woensdag: 10:00 - 18:00', 'donderdag: 10:00 - 18:00', 'vrijdag: 10:00 - 18:00', 'zaterdag: 10:00 - 18:00', 'zondag: 10:00 - 18:00']}});
const taDetails = p => ({location_id: p.id, name: p.name, web_url: 'https://www.tripadvisor.com/Restaurant_Review-' + p.id, address_obj: {street1: p.street, city: 'Gent', address_string: p.street + ', 9000 Gent'}, latitude: String(p.lat), longitude: String(p.lon), rating: p.rating, num_reviews: p.reviews, price_level: p.price,
  cuisine: p.cuisine.map(c => ({name: c, localized_name: {japanese: 'Japans', sushi: 'Sushi', asian: 'Aziatisch', thai: 'Thais', belgian: 'Belgisch', european: 'Europees', vietnamese: 'Vietnamees', soups: 'Soepen', chinese: 'Chinees', korean: 'Koreaans'}[c] || c})),
  ranking_data: {ranking_string: '#' + (12 + +p.id % 100) + ' van 850 restaurants in Gent'}, description: p.name + ' serveert verse gerechten in een gezellig interieur; reserveren aangeraden in het weekend.',
  hours: {periods: [0,1,2,3,4,5,6].map(d => ({open: {day: d, time: '1130'}, close: {day: d, time: '2230'}})), weekday_text: ['maandag: 11:30 - 22:30', 'dinsdag: 11:30 - 22:30', 'woensdag: 11:30 - 22:30', 'donderdag: 11:30 - 22:30', 'vrijdag: 11:30 - 23:00', 'zaterdag: 11:30 - 23:00', 'zondag: 11:30 - 22:30']}});
(async () => {
  const browser = await chromium.launch();
  const context = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, geolocation: { latitude: 51.0543, longitude: 3.7174, accuracy: 9 }, permissions: ['geolocation'], locale: 'nl-BE' });
  const page = await context.newPage();
  page.on('pageerror', e => console.log('PAGEERROR', e.message));
  page.on('console', m => { if (m.type() === 'error' || m.type() === 'warning') console.log('CONSOLE', m.type(), m.text().slice(0, 200)); });
  await page.addInitScript(() => {
    window.__sent = []; const store = {cfg: JSON.stringify({takey: 'test-key', rsource: 'ta', rcuisine: 'asian', rmin: '4', rdist: '5'})};
    window.miraCompanion = {
      setSnapshotForApp: async v => { window.__sent.push(Object.assign({__t: Date.now(), __snap: true}, v)); },
      sendToApp: async v => { window.__sent.push(Object.assign({__t: Date.now()}, v)); },
      onFromApp: h => { window.__fromApp = h; return () => {}; },
      storeGet: async k => store[k] ?? null, storeSet: async (k, v) => { store[k] = v; }, storeRemove: async k => { delete store[k]; },
    };
    const synth = { speak: u => { (window.__spoken = window.__spoken || []).push(u.text); }, cancel(){}, resume(){}, getVoices: () => [], pending: false, onvoiceschanged: null };
    Object.defineProperty(window, 'speechSynthesis', { value: synth, configurable: true });
    Object.defineProperty(window, 'SpeechSynthesisUtterance', { value: function(t){ this.text = t; }, configurable: true });
  });
  const log = [];
  await page.route('**/*', route => {
    const u = route.request().url();
    if (u.startsWith('file:')) return route.continue();
    log.push(u.slice(0, 90));
    if (u.includes('valhalla1.openstreetmap.de/route')) return route.fulfill({ json: valhalla });
    if (u.includes('photon.komoot.io/api')) return route.fulfill({ body: fx('photon.json'), contentType: 'application/json' });
    if (u.includes('nominatim.openstreetmap.org/reverse')) return route.fulfill({ body: fx('nominatim-rev.json'), contentType: 'application/json' });
    if (u.includes('nominatim.openstreetmap.org/search')) return route.fulfill({ body: fx('nominatim.json'), contentType: 'application/json' });
    if (u.includes('overpass')) return route.fulfill({ json: overpassFixture() });
    if (u.includes('api.content.tripadvisor.com')) {
      const sights = u.includes('category=attractions');
      if (u.includes('nearby_search') || u.includes('/location/search')) return route.fulfill({ json: { data: (sights ? TA_SIGHTS : TA_PLACES).map(p => ({location_id: p.id, name: p.name, distance: '0.5', address_obj: {address_string: p.street + ', Gent'}})) } });
      const m = /location\/(\d+)\/details/.exec(u); const p = m && TA_PLACES.find(x => x.id === m[1]); const q = m && TA_SIGHTS.find(x => x.id === m[1]);
      return p ? route.fulfill({ json: taDetails(p) }) : q ? route.fulfill({ json: taSight(q) }) : route.fulfill({ status: 404, body: '{}' });
    }
    if (u.includes('tile.openstreetmap.org')) return route.continue().catch(() => route.fulfill({ body: fx('tile.png'), contentType: 'image/png' }));
    return route.fulfill({ status: 404, body: '' });
  });
  await page.goto('file://' + path.resolve(htmlPath));
  await page.waitForFunction(() => window.__wegwijs && window.__wegwijs.S.fix, null, { timeout: 15000 });
  await page.waitForTimeout(800);
  const shot = async name => { await page.screenshot({ path: path.join(outDir, name + '.png'), fullPage: true }); };
  await shot('phone-01-start');
  // Destination search and start.
  await page.fill('#q', 'Sint-Pietersstation Gent'); await page.click('#btn-search');
  await page.waitForSelector('#q-results button'); await page.click('#q-results button');
  await page.click('#btn-start');
  await page.waitForFunction(() => window.__sent.some(m => m.type === 'route'), null, { timeout: 15000 });
  await page.waitForFunction(() => window.__sent.some(m => m.type === 'streets'), null, { timeout: 15000 }).catch(() => console.log('no streets message'));
  await page.waitForTimeout(1500);
  await page.evaluate(() => window.__wegwijs.map.fit());
  await page.waitForTimeout(2500);
  await shot('phone-02-route');
  // Walk the route: fixes at chosen distances along it (metres).
  const drive = async along => page.evaluate(along => {
    const W = window.__wegwijs, r = W.S.nav.route; let i = 0; while (i < r.cum.length - 2 && r.cum[i + 1] < along) i++;
    const a = r.pts[i], b = r.pts[i + 1]; const t = Math.max(0, Math.min(1, (along - r.cum[i]) / ((r.cum[i + 1] - r.cum[i]) || 1)));
    const lat = a[0] + (b[0] - a[0]) * t, lon = a[1] + (b[1] - a[1]) * t;
    const y = Math.sin((b[1] - a[1]) * Math.PI / 180) * Math.cos(b[0] * Math.PI / 180), x = Math.cos(a[0] * Math.PI / 180) * Math.sin(b[0] * Math.PI / 180) - Math.sin(a[0] * Math.PI / 180) * Math.cos(b[0] * Math.PI / 180) * Math.cos((b[1] - a[1]) * Math.PI / 180);
    const brg = (Math.atan2(y, x) * 180 / Math.PI + 360) % 360;
    W.S.lastNavSent = 0; W.acceptFix({lat, lon, acc: 7, speed: 1.4, heading: brg, t: Date.now()}); W.publishNav(true);
  }, along);
  const marks = {};
  const snapAt = async (name, along) => { await drive(along); await page.waitForTimeout(250); const sent = await page.evaluate(() => window.__sent); marks[name] = sent.filter(m => m.type === 'nav').pop(); };
  await snapAt('nav-start', 2);
  await snapAt('nav-approach', 55);        // before the left turn onto Poel (at ~65 m)
  await snapAt('nav-turn', 118);           // just before the second left turn
  await snapAt('nav-long', 420);           // on the long stretch along Oude Houtlei
  await snapAt('nav-mid', 1400);
  await shot('phone-03-onderweg');
  // Off route: 80 m east of the route.
  await page.evaluate(() => { const W = window.__wegwijs; const f = W.S.fix; for (let k = 0; k < 4; k++) { W.S.lastNavSent = 0; W.acceptFix({lat: f.lat, lon: f.lon + 0.0012, acc: 7, speed: 1.4, heading: 90, t: Date.now() + k * 4000}); } });
  await page.waitForTimeout(800);
  marks['nav-offroute'] = (await page.evaluate(() => window.__sent)).filter(m => m.type === 'nav').pop();
  await snapAt('nav-near-end', 2650);
  await snapAt('nav-arrived', 2698);
  await page.waitForTimeout(300);
  await shot('phone-04-aangekomen');
  // Restaurants (Tripadvisor fixture).
  await page.click('#btn-rsearch');
  await page.waitForFunction(() => window.__sent.some(m => m.type === 'places'), null, { timeout: 15000 });
  await page.waitForTimeout(500);
  await shot('phone-05-restaurants');
  // Choose from the glasses: go to the first proposal.
  await page.evaluate(() => window.__fromApp({ type: 'go', i: 0 }));
  await page.waitForFunction(() => window.__sent.filter(m => m.type === 'route').length >= 2, null, { timeout: 15000 });
  await page.waitForTimeout(500);
  // Spoken commands from the glasses.
  await page.evaluate(() => window.__fromApp({ type: 'say', text: 'Wat is er te zien in de buurt?' }));
  await page.waitForFunction(() => window.__sent.filter(m => m.type === 'places').length >= 2, null, { timeout: 15000 });
  await page.waitForTimeout(400);
  await shot('phone-06-bezienswaardigheden');
  await page.evaluate(() => window.__fromApp({ type: 'say', text: 'Zoek een goed Aziatisch restaurant binnen 3 km.' }));
  await page.waitForFunction(() => window.__sent.filter(m => m.type === 'places').length >= 3, null, { timeout: 15000 });
  await page.evaluate(() => window.__wegwijs.stopNavigation());
  await page.evaluate(() => window.__fromApp({ type: 'say', text: 'Fiets naar Sint-Pietersstation Gent' }));
  await page.waitForFunction(() => window.__sent.filter(m => m.type === 'route').length >= 3, null, { timeout: 15000 });
  await page.waitForTimeout(400);
  await page.evaluate(() => window.__fromApp({ type: 'say', text: 'blabla' }));
  await page.waitForTimeout(400);
  const sent = await page.evaluate(() => window.__sent);
  const spoken = await page.evaluate(() => window.__spoken || []);
  fs.writeFileSync(path.join(outDir, 'messages.json'), JSON.stringify(sent, null, 1));
  fs.writeFileSync(path.join(outDir, 'marks.json'), JSON.stringify(marks, null, 1));
  fs.writeFileSync(path.join(outDir, 'spoken.txt'), spoken.join('\n'));
  const enc = s => Buffer.byteLength(JSON.stringify(s));
  const strip = m => { const c = Object.assign({}, m); delete c.__t; delete c.__snap; return c; };
  console.log('messages:', sent.length, 'types:', sent.map(m => m.type).join(','));
  for (const t of ['route', 'streets', 'places']) { const m = sent.filter(x => x.type === t); console.log(t, m.map(x => enc(strip(x)) + 'B').join(' '), m.length ? 'lines=' + m[0].d.split('\n').length : ''); }
  console.log('--- route message head:\n' + sent.find(m => m.type === 'route').d.split('\n').slice(0, 6).join('\n').slice(0, 600));
  console.log('--- nav marks:'); for (const [k, v] of Object.entries(marks)) console.log(k, '=>', v ? v.d.replace(/\n/g, ' | ') : 'none');
  console.log('--- spoken:\n' + spoken.join('\n'));
  console.log('--- toasts:', sent.filter(m => m.type === 'toast').map(m => m.d).join(' | '));
  console.log('--- places 2 (bezienswaardigheden):\n' + sent.filter(m => m.type === 'places')[1].d);
  console.log('--- last nav mode:', sent.filter(m => m.type === 'nav').pop().d.split('\n').pop());
  console.log('--- network (unique hosts):', [...new Set(log.map(u => u.split('/')[2]))].join(', '));
  await browser.close();
})().catch(e => { console.error(e); process.exit(1); });
