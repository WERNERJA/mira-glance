//! Ordered app actions. Never repeat a possibly completed side effect.
#[link(wasm_import_module="mira")]extern "C" { fn ble_request(m:i32,p:*const u8,n:i32)->i32; fn ble_response_len(i:i32)->i32; fn ble_response_read(i:i32,p:*mut u8,n:i32)->i32; }
pub struct Commands { queue:[u8;32], read:usize, count:usize, pending:i32, age:u32 }
impl Commands {
 pub const fn new()->Self{Self{queue:[0;32],read:0,count:0,pending:0,age:0}}
 pub fn push(&mut self,value:u8){if self.count<32{self.queue[(self.read+self.count)%32]=value;self.count+=1;}}
 pub unsafe fn tick(&mut self,method:i32,ms:u32)->bool {
  self.tick_with(method,ms,ble_request,ble_response_len,ble_response_read)
 }
 pub unsafe fn tick_with(&mut self,method:i32,ms:u32,
  ble_request:unsafe extern "C" fn(i32,*const u8,i32)->i32,
  ble_response_len:unsafe extern "C" fn(i32)->i32,
  ble_response_read:unsafe extern "C" fn(i32,*mut u8,i32)->i32)->bool {
  let mut done=false;
  if self.pending>0{self.age=self.age.saturating_add(ms);let n=ble_response_len(self.pending);if n>=0{let mut b=[0;384];ble_response_read(self.pending,b.as_mut_ptr(),384);self.pending=0;done=true;}else if n != -1 || self.age>3000{self.pending=0;done=true;}}
  if self.pending<=0 && self.count>0{self.pending=ble_request(method,&self.queue[self.read],1);if self.pending>0{self.read=(self.read+1)%32;self.count-=1;self.age=0;}}
  done
 }
}
