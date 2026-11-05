#![no_std]
#![allow(static_mut_refs)]
#[path = "../Shared/companion.rs"] mod companion;
#[path = "../Shared/content.rs"] mod content;

app_icon!(content::app_icon::Icon::new()
    .circle(16,16,12).line(10,16,22,16).line(16,10,16,22).0);

#[link(wasm_import_module="mira")]
extern "C" {fn begin();fn text(x:i32,y:i32,w:i32,h:i32,c:i32,font:i32,a:i32,s:i32,p:*const u8,n:i32);fn present();}
#[panic_handler]fn panic(_: &core::panic::PanicInfo)->!{loop{}}
static mut BRIDGE:companion::Companion=companion::Companion::new(b"Companion Counter");
static mut N:u32=0;
static mut NEED_SEND:bool=false;
unsafe fn render(){let mut label=[0u8;32];label[..6].copy_from_slice(b"Count ");let n=digits(N,&mut label[6..]);begin();text(24,100,592,40,15,1,0,24,label.as_ptr(),(n+6) as i32);content::cursor(614, 440, true);present();}
fn digits(mut n:u32,out:&mut[u8])->usize{let mut reversed=[0u8;10];let mut count=0;loop{reversed[count]=b'0'+(n%10) as u8;count+=1;n/=10;if n==0{break;}}for i in 0..count{out[i]=reversed[count-1-i];}count}
#[no_mangle]pub unsafe extern "C" fn app_init(){N=0;BRIDGE=companion::Companion::new(b"Companion Counter");NEED_SEND=true;render();}
#[no_mangle]pub unsafe extern "C" fn app_render(ms:i32){
 let shell_dirty=content::context_changed();
 if let Some(message)=BRIDGE.tick(ms.max(0) as u32){
  if message==b"{\"type\":\"bump\"}" {N=N.saturating_add(1);NEED_SEND=true;render();}
  if message==b"{\"type\":\"sync\"}" {NEED_SEND=true;}
 }
 if shell_dirty{render();}
 if NEED_SEND{let mut json=[0u8;64];let head=b"{\"type\":\"count\",\"n\":";json[..head.len()].copy_from_slice(head);let n=digits(N,&mut json[head.len()..]);let end=head.len()+n;json[end]=b'}';if BRIDGE.send(&json[..end+1]){NEED_SEND=false;}}
}
#[no_mangle]pub unsafe extern "C" fn app_event(e:i32){if e==0{N=N.saturating_add(1);NEED_SEND=true;render();}}

// The host releases this instance and its owned resources after this callback.
#[no_mangle] pub extern "C" fn app_stop() {}
