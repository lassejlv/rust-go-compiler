use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum State { Closed, Listen, SynSent { seq: u32 }, SynReceived { seq: u32, ack: u32 }, Established { seq: u32, ack: u32, buffered: Vec<u8> }, FinWait { pending: usize }, TimeWait(u8) }

#[derive(Debug, Clone, Copy)]
enum Event<'a> { Open { passive: bool }, Syn(u32), SynAck(u32, u32), Ack(u32), Data(&'a [u8]), Close, Fin, Tick }

#[derive(Debug, PartialEq)]
enum Action { Send(String), Deliver(Vec<u8>), Drop(&'static str) }

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            State::Closed => "CLOSED".to_string(), State::Listen => "LISTEN".into(),
            State::SynSent { seq } => format!("SYN_SENT(seq={})", seq),
            State::SynReceived { seq, ack } => format!("SYN_RCVD({}/{})", seq, ack),
            State::Established { seq, ack, buffered } => format!("ESTABLISHED({}/{}, buf={})", seq, ack, buffered.len()),
            State::FinWait { pending } => format!("FIN_WAIT({})", pending),
            State::TimeWait(n) => format!("TIME_WAIT({})", n),
        };
        f.pad(&s)
    }
}

struct Conn { state: State, isn: u32, log: Vec<String> }

impl Conn {
    fn handle(&mut self, ev: Event) -> Vec<Action> {
        let old = std::mem::replace(&mut self.state, State::Closed);
        let (new, actions) = match (old, ev) {
            (State::Closed, Event::Open { passive: true }) => (State::Listen, vec![]),
            (State::Closed, Event::Open { passive: false }) => (State::SynSent { seq: self.isn }, vec![Action::Send(format!("SYN {}", self.isn))]),
            (State::Listen, Event::Syn(s)) => (State::SynReceived { seq: self.isn, ack: s + 1 }, vec![Action::Send(format!("SYN-ACK {} {}", self.isn, s + 1))]),
            (State::SynSent { seq }, Event::SynAck(s, a)) if a == seq + 1 => (State::Established { seq: a, ack: s + 1, buffered: vec![] }, vec![Action::Send(format!("ACK {}", s + 1))]),
            (st @ State::SynSent { .. }, Event::SynAck(..)) => (st, vec![Action::Drop("bad ack")]),
            (State::SynReceived { seq, ack }, Event::Ack(a)) if a == seq + 1 => (State::Established { seq: a, ack, buffered: vec![] }, vec![]),
            (State::Established { seq, ack, mut buffered }, Event::Data(d)) => {
                buffered.extend_from_slice(d);
                let mut acts = vec![Action::Send(format!("ACK {}", ack + d.len() as u32))];
                if let Some(pos) = buffered.iter().position(|&b| b == b'\n') {
                    let line: Vec<u8> = buffered.drain(..=pos).collect();
                    acts.push(Action::Deliver(line));
                }
                (State::Established { seq, ack: ack + d.len() as u32, buffered }, acts)
            }
            (State::Established { buffered, .. }, Event::Close) => (State::FinWait { pending: buffered.len() }, vec![Action::Send("FIN".into())]),
            (State::Established { buffered, .. }, Event::Fin) => {
                let mut acts = vec![];
                if !buffered.is_empty() { acts.push(Action::Deliver(buffered)); }
                acts.push(Action::Send("ACK FIN".into()));
                (State::TimeWait(2), acts)
            }
            (State::FinWait { .. }, Event::Fin) => (State::TimeWait(2), vec![Action::Send("ACK FIN".into())]),
            (State::TimeWait(0), Event::Tick) => (State::Closed, vec![]),
            (State::TimeWait(n), Event::Tick) => (State::TimeWait(n - 1), vec![]),
            (st, Event::Tick) => (st, vec![]),
            (st, e) => { let msg = format!("ignored {:?} in {}", e, st); self.log.push(msg); (st, vec![Action::Drop("unexpected")]) }
        };
        self.log.push(format!("{:>28} -> {:?}", new.to_string(), actions));
        self.state = new;
        actions
    }
}

fn main() {
    let mut client = Conn { state: State::Closed, isn: 1000, log: vec![] };
    let mut server = Conn { state: State::Closed, isn: 5000, log: vec![] };
    server.handle(Event::Open { passive: true });
    client.handle(Event::Open { passive: false });
    server.handle(Event::Syn(1000));
    client.handle(Event::SynAck(5000, 999));
    client.handle(Event::SynAck(5000, 1001));
    server.handle(Event::Ack(5001));
    server.handle(Event::Ack(5001));
    let chunks: [&[u8]; 4] = [b"hel", b"lo\nwor", b"ld", b"\npartial"];
    let mut delivered = vec![];
    for c in chunks {
        for a in server.handle(Event::Data(c)) { if let Action::Deliver(d) = a { delivered.push(String::from_utf8(d).unwrap()); } }
    }
    for a in server.handle(Event::Fin) { if let Action::Deliver(d) = a { delivered.push(String::from_utf8_lossy(&d).into_owned()); } }
    client.handle(Event::Close);
    client.handle(Event::Data(b"late"));
    client.handle(Event::Fin);
    for _ in 0..4 { client.handle(Event::Tick); server.handle(Event::Tick); }
    println!("delivered {:?}", delivered);
    println!("client log:"); for l in &client.log { println!("  {}", l); }
    println!("server log:"); for l in &server.log { println!("  {}", l); }
    println!("[{:<12}] [{:>30}]", State::Listen, State::Established { seq: 1, ack: 2, buffered: vec![0; 3] });
}
