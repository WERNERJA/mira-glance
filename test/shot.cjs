const { chromium } = require('playwright'); const fs = require('fs'); const path = require('path');
(async () => {
  const dir = process.argv[2]; const b = await chromium.launch(); const p = await b.newPage({ viewport: { width: 640, height: 480 }, deviceScaleFactor: 1 });
  const files = fs.readdirSync(dir).filter(f => f.endsWith('.svg')).sort();
  for (const f of files) { await p.goto('file://' + path.resolve(dir, f)); await p.screenshot({ path: path.join(dir, f.replace('.svg', '.png')) }); }
  // contact sheet
  const imgs = files.map(f => `<figure><img src="${f}"><figcaption>${f.replace('.svg','')}</figcaption></figure>`).join('');
  fs.writeFileSync(path.join(dir, 'sheet.html'), `<body style="margin:0;background:#111;color:#aaa;font:14px sans-serif;display:grid;grid-template-columns:repeat(3,640px);gap:12px;padding:12px">${imgs}</body>`);
  const s = await b.newPage({ viewport: { width: 1980, height: 1100 } }); await s.goto('file://' + path.resolve(dir, 'sheet.html')); await s.screenshot({ path: path.join(dir, 'sheet.png'), fullPage: true });
  await b.close();
})();
