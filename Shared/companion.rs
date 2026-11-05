//! The HTML companion's existing JSON event API over the WASM phone bridge.
//! Call tick each frame; handle its returned message once. send queues one JSON
//! response (up to 4096 bytes), fragmented and retried with a stable message ID.
#[path = "casa_data.rs"] mod data;
#[link(wasm_import_module = "mira")]
extern "C" {
    fn ble_request(method:i32, payload:*const u8, len:i32)->i32;
    fn ble_response_len(id:i32)->i32;
    fn ble_response_read(id:i32, output:*mut u8, cap:i32)->i32;
}
pub struct Companion<const IN:usize=4096,const OUT:usize=4096> {
    incoming:data::Snapshot<IN>, delivered:u32, ack:u32,
    name:&'static [u8], epoch:u32, identity_pending:i32, identity_age:u32,
    outgoing:[u8;OUT], len:usize, offset:usize, id:u32,
    pending:i32, pending_ack:bool, pending_count:usize, age:u32,
    pub failed:bool,
}
impl<const IN:usize,const OUT:usize> Companion<IN,OUT> {
    pub const fn new(name:&'static [u8])->Self { Self {incoming:data::Snapshot::new(),delivered:0,ack:0,
        name,epoch:0,identity_pending:0,identity_age:5_000,
        outgoing:[0;OUT],len:0,offset:0,id:0,pending:0,pending_ack:false,pending_count:0,age:0,failed:false} }
    pub fn send(&mut self,json:&[u8])->bool {
        if self.len>0 || json.is_empty() || json.len()>OUT || json.len()>4096 {return false;}
        self.outgoing[..json.len()].copy_from_slice(json); self.len=json.len();self.offset=0;
        self.id=self.id.wrapping_add(1).max(1);true
    }
    pub unsafe fn tick(&mut self,elapsed:u32)->Option<&[u8]> {
        self.identity_age=self.identity_age.saturating_add(elapsed);
        if self.identity_pending>0 {
            let available=ble_response_len(self.identity_pending);
            if available>=0 {
                let mut reply=[0u8;384];
                let n=ble_response_read(self.identity_pending,reply.as_mut_ptr(),384);
                self.identity_pending=0;
                if n==4 {
                    let epoch=u32::from_le_bytes(reply[..4].try_into().unwrap());
                    if epoch!=self.epoch {
                        self.incoming.reset();self.delivered=0;self.ack=0;
                        self.pending=0;self.offset=0;self.epoch=epoch;
                    }
                }
            } else if available != -1 || self.identity_age>=3_000 {self.identity_pending=0;}
        }
        if self.identity_pending<=0 && self.identity_age>=5_000 {
            self.identity_pending=ble_request(0x0064,self.name.as_ptr(),self.name.len() as i32);
            self.identity_age=0;
        }
        if self.epoch==0 {return None;}
        self.incoming.tick(0x0062,elapsed,100);
        if self.pending>0 {
            self.age=self.age.saturating_add(elapsed);
            let n=ble_response_len(self.pending);
            if n>=0 {
                let mut reply=[0u8;384];let n=ble_response_read(self.pending,reply.as_mut_ptr(),384);
                self.pending=0;
                if n>=0 {
                    self.failed=false;
                    if self.pending_ack {self.ack=0;self.incoming.refresh();}
                    else {self.offset+=self.pending_count;if self.offset==self.len{self.len=0;self.offset=0;}}
                } else {self.failed=true;}
            } else if n != -1 || self.age>=3_000 {self.pending=0;self.failed=true;}
        }
        let sequence=data::number(self.incoming.field(0)) as u32;
        let fresh=sequence!=0 && sequence!=self.delivered && self.ack==0;
        if fresh {self.delivered=sequence;self.ack=sequence;}
        if self.pending<=0 && (self.ack!=0 || self.len>0) {
            let mut packet=[0u8;384];let count;
            self.pending_ack=self.ack!=0;
            if self.pending_ack {packet[0]=0;packet[1..5].copy_from_slice(&self.ack.to_le_bytes());count=5;}
            else {
                packet[0]=1;packet[1..5].copy_from_slice(&self.id.to_le_bytes());
                packet[5..7].copy_from_slice(&(self.offset as u16).to_le_bytes());
                packet[7..9].copy_from_slice(&(self.len as u16).to_le_bytes());
                self.pending_count=(self.len-self.offset).min(375);count=9+self.pending_count;
                packet[9..count].copy_from_slice(&self.outgoing[self.offset..self.offset+self.pending_count]);
            }
            self.pending=ble_request(0x0063,packet.as_ptr(),count as i32);self.age=0;
            if self.pending<=0 {self.failed=true;}
        }
        if fresh {Some(self.incoming.field(1))} else {None}
    }
}

/// Latest-value duplex channel for compact, time-sensitive controls/state.
/// Pair with miraCompanion.setLatestForApp (380-byte JSON limit). Replaces
/// unsent controls and allows one BLE exchange in flight, so delays do not
/// build a backlog. Ordered messages should continue to use Companion.
pub struct Realtime {
    name: &'static [u8], epoch: u32, identity: i32, identity_age: u32,
    pending: i32, age: u32, cadence: u32, id: u32, revision: u32,
    outgoing: [u8;379], len: usize, incoming: [u8;384],
    pub failed: bool,
}
impl Realtime {
    pub const fn new(name: &'static [u8]) -> Self {
        Self { name, epoch:0, identity:0, identity_age:5000, pending:0, age:0,
            cadence:50, id:0, revision:0, outgoing:[0;379], len:0, incoming:[0;384], failed:false }
    }
    pub fn send(&mut self, json: &[u8]) -> bool {
        if json.is_empty() || json.len()>379 {return false;}
        self.outgoing[..json.len()].copy_from_slice(json);self.len=json.len();true
    }
    pub unsafe fn tick(&mut self, elapsed:u32) -> Option<&[u8]> {
        self.identity_age=self.identity_age.saturating_add(elapsed);
        self.cadence=self.cadence.saturating_add(elapsed);
        if self.identity>0 {
            let n=ble_response_len(self.identity);
            if n>=0 {
                let mut reply=[0u8;384];let n=ble_response_read(self.identity,reply.as_mut_ptr(),384);self.identity=0;
                if n==4 {
                    let epoch=u32::from_le_bytes(reply[..4].try_into().unwrap());
                    if epoch!=self.epoch {self.epoch=epoch;self.revision=0;self.pending=0;}
                }
            } else if n != -1 || self.identity_age>=1500 {self.identity=0;}
        }
        if self.identity<=0 && self.identity_age>=5000 {
            self.identity=ble_request(0x64,self.name.as_ptr(),self.name.len() as i32);self.identity_age=0;
        }
        if self.epoch==0 {return None;}
        let mut fresh=0;
        if self.pending>0 {
            self.age=self.age.saturating_add(elapsed);
            let n=ble_response_len(self.pending);
            if n>=0 {
                let n=ble_response_read(self.pending,self.incoming.as_mut_ptr(),384);self.pending=0;
                self.failed=n<4;
                if n>=4 {
                    let revision=u32::from_le_bytes(self.incoming[..4].try_into().unwrap());
                    if revision!=0 && revision!=self.revision && n>4 {self.revision=revision;fresh=n as usize;}
                }
            } else if n != -1 || self.age>=1000 {self.pending=0;self.failed=true;}
        }
        if self.pending<=0 && self.len>0 && self.cadence>=50 {
            let mut packet=[0u8;384];packet[0]=2;self.id=self.id.wrapping_add(1).max(1);
            packet[1..5].copy_from_slice(&self.id.to_le_bytes());packet[5..5+self.len].copy_from_slice(&self.outgoing[..self.len]);
            self.pending=ble_request(0x63,packet.as_ptr(),(self.len+5) as i32);self.age=0;self.cadence=0;
            if self.pending<=0 {self.failed=true;}
        }
        if fresh>0 {Some(&self.incoming[4..fresh])}else{None}
    }
}
