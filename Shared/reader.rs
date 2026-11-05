// Shared WASM document reader. The phone supplies text chunks, not layout.
use super::*;
const ADV24: [u8; 95] = [
    6, 7, 8, 15, 13, 19, 17, 4, 8, 8, 12, 14, 5, 7, 5, 8, 15, 10, 13, 12, 15, 14, 14, 13, 14, 14,
    7, 7, 14, 14, 14, 12, 24, 16, 15, 15, 17, 14, 13, 17, 18, 6, 11, 15, 13, 21, 18, 18, 15, 18,
    16, 14, 14, 18, 16, 24, 15, 14, 15, 8, 8, 8, 14, 8, 6, 13, 15, 12, 15, 13, 9, 14, 13, 5, 6,
    12, 4, 22, 14, 14, 15, 14, 9, 11, 10, 14, 13, 20, 12, 13, 12, 8, 6, 8, 14,
];

static mut DOC:content::data::Snapshot<4096>=content::data::Snapshot::new();
static mut BYTES:[u8;2400]=[0;2400];static mut SIZE:usize=0;
static mut STARTS:[u16;2048]=[0;2048];static mut LINES:usize=0;
static mut TOP:usize=0;static mut CURRENT:i32=-1;static mut VERSION:[u8;64]=[0;64];static mut VLEN:usize=0;
static mut REVERSE:bool=false;static mut WAIT:i32=0;static mut WAIT_MS:u32=0;static mut ERROR:bool=false;
static mut TIMER_MS:u32=0;static mut RUNNING:bool=false;
static mut POSITION:i32=0;
fn advance(b:u8)->i32 {if (32..=126).contains(&b){ADV24[(b-32) as usize] as i32}else if b&0xc0==0x80 || b<32 {0}else{24}}
unsafe fn wrap(){
 LINES=0;let mut start=0;
 while start<SIZE && LINES<STARTS.len() {
  STARTS[LINES]=start as u16;LINES+=1;
  let mut i=start;let mut width=0;let mut space=None;
  while i<SIZE {
   let b=BYTES[i];if b==b'\n' {i+=1;break;}
   if b==b' ' {space=Some(i);}
   let added=advance(b);
   if width+added>608 && i>start {i=space.filter(|p|*p>start).map(|p|p+1).unwrap_or(i);break;}
   width+=added;i+=1;
  }
  start=i.max(start+1);
 }
}
unsafe fn apply(){
 let chunk=content::data::number(DOC.field(4)) as i32;let version=DOC.field(0);
 if chunk==CURRENT && version==&VERSION[..VLEN] {return;}
 let same=version==&VERSION[..VLEN];VLEN=version.len().min(64);VERSION[..VLEN].copy_from_slice(&version[..VLEN]);
 CURRENT=chunk;SIZE=DOC.field(3).len().min(BYTES.len());BYTES[..SIZE].copy_from_slice(&DOC.field(3)[..SIZE]);wrap();
 TOP=if same && REVERSE {LINES.saturating_sub(11)}else{0};POSITION=TOP as i32*28;REVERSE=false;
 if !same {TIMER_MS=0;RUNNING=false;}
}
unsafe fn tx(x:i32,y:i32,w:i32,h:i32,font:i32,value:&[u8]){let mut n=0;let mut width=0;
 while n<value.len(){let b=value[n];let len=if b<128{1}else if b&0xe0==0xc0{2}else if b&0xf0==0xe0{3}else{4};let add=advance(b);if width+add>w || b==b'\n'{break;}width+=add;n=(n+len).min(value.len());}
 text(x,y,w,h,15,font,0,28,value.as_ptr(),n as i32);}
unsafe fn number(mut n:u32,out:&mut[u8])->usize {let mut temp=[0u8;12];let mut len=0;loop{temp[len]=b'0'+(n%10) as u8;len+=1;n/=10;if n==0{break;}}for i in 0..len{out[i]=temp[len-1-i];}len}
unsafe fn render(){
 begin();tx(20,15,600,38,0,DOC.field(1));tx(20,52,600,28,1,DOC.field(2));
 if SIZE==0 {tx(24,175,592,100,0,b"Choose content on your phone");}
 else {
  let first=(POSITION/28).max(0) as usize;
  for index in first..(first+13).min(LINES){let from=STARTS[index] as usize;let end=if index+1<LINES{STARTS[index+1] as usize}else{SIZE};let mut to=end;while to>from && (BYTES[to-1]==b'\n'||BYTES[to-1]==b' '){to-=1;}
   let y=88+index as i32*28-POSITION;if y>=84 && y<404 {tx(16,y,608,28,0,&BYTES[from..to]);}
  }
 }
 let mut label=[0u8;96];let mut n=0;
 for b in b"Part " {label[n]=*b;n+=1;}n+=number((CURRENT+1).max(0) as u32,&mut label[n..]);
 label[n]=b'/';n+=1;n+=number(content::data::number(DOC.field(5)).max(1) as u32,&mut label[n..]);
 if TIMED {for b in b"  Time "{label[n]=*b;n+=1;}n+=number(TIMER_MS/1000,&mut label[n..]);label[n]=b's';n+=1;}
 tx(20,416,600,28,1,&label[..n]);
 tx(20,449,600,24,1,if ERROR{b"Request failed; swipe to retry"}else if DOC.failed{b"Phone disconnected"}else if WAIT>0{b"Loading next part..."}else if DOC.field(6)!=b"Ready"{DOC.field(6)}else if TIMED {if RUNNING{b"Swipe to scroll / tap to pause timer"}else{b"Swipe to scroll / tap to start timer"}}else{b"Swipe to read / tap for next page"});
 content::cursor(610, 420, true);present();
}
unsafe fn shift(direction:i32){
 if WAIT>0{return;}
 let max=LINES.saturating_sub(11);
 let next=(TOP as i32+direction*6).clamp(0,max as i32) as usize;
 if next!=TOP {TOP=next;return;}
 let part=CURRENT+direction;let count=content::data::number(DOC.field(5)) as i32;
 if part>=0 && part<count {let mut body=[0u8;69];body[1..5].copy_from_slice(&(part as u32).to_le_bytes());body[5..5+VLEN].copy_from_slice(&VERSION[..VLEN]);WAIT=ble_request(METHOD+1,body.as_ptr(),(5+VLEN) as i32);WAIT_MS=0;ERROR=WAIT<=0;REVERSE=direction<0;}
}
pub unsafe fn init(){render();}
pub unsafe fn tick(ms:i32){
 content::context_changed();let dt=ms.clamp(0,250) as u32;
 if DOC.tick(METHOD,dt,250){apply();}
 if WAIT>0 {WAIT_MS+=dt;let len=ble_response_len(WAIT);if len>=0{let mut out=[0u8;384];ERROR=ble_response_read(WAIT,out.as_mut_ptr(),384)<0;WAIT=0;DOC.refresh();}else if len != -1 || WAIT_MS>=3000 {ERROR=true;WAIT=0;}}
 if RUNNING && !content::sidebar_open() && !content::overlay_visible(){TIMER_MS=TIMER_MS.saturating_add(dt);}
 let target=TOP as i32*28;let speed=(dt as i32*560/1000).max(1);POSITION+= (target-POSITION).clamp(-speed,speed);render();
}
pub unsafe fn event(e:i32){
match e{1=>shift(-1),2=>shift(1),0 if TIMED=>RUNNING=!RUNNING,0=>shift(1),_=>{}}render();}
