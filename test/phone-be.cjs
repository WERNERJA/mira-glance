// Loads the packaged companion in Chromium with a mocked miraCompanion and
// mocked network, captures the snapshot it publishes, and screenshots the UI.
const { chromium } = require('playwright');
const fs = require('fs');
const htmlPath = process.argv[2], outDir = process.argv[3];
const pad = n => String(n).padStart(2, '0');
const now = new Date();
const ymd = d => d.getFullYear() + pad(d.getMonth() + 1) + pad(d.getDate());
const add = (d, n) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
const today = add(now, 0);
const hourIso = (d, h) => `${d.getFullYear()}-${pad(d.getMonth()+1)}-${pad(d.getDate())}T${pad(h)}:00`;
// --- weather fixture ---
const hours = [], temps = [], codes = [], pp = [], isday = [];
for (let d = 0; d < 4; d++) for (let h = 0; h < 24; h++) { const dt = add(today, d); hours.push(hourIso(dt, h)); temps.push(8 + 4 * Math.sin((h - 9) / 24 * 2 * Math.PI)); codes.push([0,1,2,3,61,80,3,2][(h + d) % 8]); pp.push(((h * 7) % 60)); isday.push(h >= 8 && h < 17 ? 1 : 0); }
const days = [0,1,2,3].map(i => { const d = add(today, i); return `${d.getFullYear()}-${pad(d.getMonth()+1)}-${pad(d.getDate())}`; });
const weather = { current: { time: hourIso(today, now.getHours()).slice(0, 14) + '15', temperature_2m: 11.4, apparent_temperature: 9.2, weather_code: 2, wind_speed_10m: 18.3, is_day: 1 },
  hourly: { time: hours, temperature_2m: temps, weather_code: codes, precipitation_probability: pp, is_day: isday },
  daily: { time: days, weather_code: [2, 61, 3, 0], temperature_2m_max: [12.6, 10.1, 9.4, 11.8], temperature_2m_min: [6.2, 5.5, 3.1, 2.9], precipitation_probability_max: [20, 80, 10, 0] } };
// --- calendar fixture (TZID, all-day, weekly recurrence with EXDATE, UTC) ---
const inH = (h, m = 0, d = 0) => { const t = add(today, d); return `${ymd(t)}T${pad(h)}${pad(m)}00`; };
const later = new Date(now.getTime() + 2 * 3600000);
const ics = ['BEGIN:VCALENDAR','VERSION:2.0',
 'BEGIN:VEVENT','UID:a','DTSTART;TZID=Europe/Amsterdam:' + `${ymd(later)}T${pad(later.getHours())}3000`,'DTEND;TZID=Europe/Amsterdam:' + `${ymd(later)}T${pad(later.getHours()+1)}0000`,'SUMMARY:Overleg met het team\\, Q4 planning','LOCATION:Kantoor Utrecht','END:VEVENT',
 'BEGIN:VEVENT','UID:b','DTSTART;VALUE=DATE:' + ymd(add(today, 1)),'DTEND;VALUE=DATE:' + ymd(add(today, 2)),'SUMMARY:Verjaardag Anna','END:VEVENT',
 'BEGIN:VEVENT','UID:c','DTSTART;TZID=W. Europe Standard Time:' + inH(9, 15, -14),'DURATION:PT45M','RRULE:FREQ=WEEKLY;BYDAY=' + ['SU','MO','TU','WE','TH','FR','SA'][add(today,1).getDay()],'SUMMARY:Wekelijkse sync','LOCATION:Teams','END:VEVENT',
 'BEGIN:VEVENT','UID:d','DTSTART:' + inH(23, 0, -3).replace(/T(\d{2})/, 'T$1') ,'DTEND:' + inH(23, 30, -3),'SUMMARY:Oud event','END:VEVENT',
 'BEGIN:VEVENT','UID:e','DTSTART;TZID=Europe/Amsterdam:' + inH(18, 0, 2),'DTEND;TZID=Europe/Amsterdam:' + inH(19, 30, 2),'SUMMARY:Tandarts','LOCATION:Mondzorg Centrum\\, Laan 12','RRULE:FREQ=DAILY;COUNT=1','END:VEVENT',
 'BEGIN:VEVENT','UID:f','DTSTART;TZID=Europe/Amsterdam:' + inH(7, 0, 0),'DTEND;TZID=Europe/Amsterdam:' + inH(23, 59, 0),'SUMMARY:Deze cancelled','STATUS:CANCELLED','END:VEVENT',
 'END:VCALENDAR'].join('\r\n');
const rss = `<?xml version="1.0"?><rss><channel><title>HLN.be</title>${[
 'Vlaamse regering keurt nieuw mobiliteitsplan goed',
 'Storm verwacht langs de kust: code geel voor Zeeland en Noord-Holland',
 'Ajax wint met 3-1 van PSV in spannende topper',
 'Nieuwe treinverbinding tussen Amsterdam en Berlijn sneller dan verwacht',
 'Onderzoek: steeds meer Nederlanders werken vier dagen per week',
 'Rijksmuseum opent tentoonstelling over de Gouden Eeuw & handel'].map(t => `<item><title><![CDATA[${t}]]></title></item>`).join('')}</channel></rss>`;
const yahoo = { '^BFX': [4321.55, 4300.1], 'ABI.BR': [58.12, 58.9], 'BTC-EUR': [61234.5, 62890.1] };
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2 });
  page.on('pageerror', e => console.log('PAGEERROR', e.message));
  page.on('console', m => { if (m.type() === 'error') console.log('CONSOLE', m.text()); });
  await page.addInitScript((icsUrl) => {
    window.__sent = []; const store = {cfg: JSON.stringify({ics: icsUrl, todos: [{text:'Boodschappen halen', done:false},{text:'Pakket terugsturen', done:false},{text:'Mail aan Pieter', done:true}]})};
    window.miraCompanion = {
      setSnapshotForApp: async v => { window.__sent.push(v); },
      sendToApp: async v => { window.__sent.push(v); },
      onFromApp: h => { window.__fromApp = h; return () => {}; },
      storeGet: async k => store[k] ?? null, storeSet: async (k, v) => { store[k] = v; }, storeRemove: async k => { delete store[k]; },
    };
  }, 'webcal://example.test/cal.ics');
  await page.route('**/*', route => {
    const u = route.request().url();
    if (u.startsWith('file:')) return route.continue();
    if (u.includes('api.open-meteo.com')) return route.fulfill({ json: weather });
    if (u.includes('example.test/cal.ics')) return route.fulfill({ body: ics, contentType: 'text/calendar' });
    if (u === 'https://www.hln.be/rss.xml') return route.fulfill({ status: 404, body: '' });
    if (u.includes('hln.be/home/rss.xml')) return route.fulfill({ body: rss, contentType: 'application/rss+xml' });
    const m = /chart\/([^?]+)/.exec(u);
    if (m) { const s = decodeURIComponent(m[1]); const v = yahoo[s]; return v ? route.fulfill({ json: { chart: { result: [{ meta: { regularMarketPrice: v[0], chartPreviousClose: v[1] } }] } } }) : route.fulfill({ status: 404, body: '' }); }
    return route.fulfill({ status: 404, body: '' });
  });
  await page.goto('file://' + require('path').resolve(htmlPath));
  await page.waitForFunction(() => window.__sent.some(m => m.d && m.d.includes('U|')), null, { timeout: 15000 });
  await page.waitForTimeout(300);
  const sent = await page.evaluate(() => window.__sent);
  const last = sent.filter(m => m.d).pop();
  fs.writeFileSync(outDir + '/snapshot.json', JSON.stringify(last));
  console.log('messages sent:', sent.length, 'json bytes:', Buffer.byteLength(JSON.stringify(last)));
  console.log(last.d);
  await page.screenshot({ path: outDir + '/phone.png', fullPage: true });
  // glasses -> phone refresh path
  await page.evaluate(() => window.__fromApp({ type: 'refresh' }));
  await page.waitForTimeout(500);
  console.log('after refresh, messages:', (await page.evaluate(() => window.__sent)).length);
  await browser.close();
})();
