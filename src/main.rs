//! SkillBoost Reflex Arena - Rust backend (axum + WebSocket).
//! One host screen shows the clues, players answer from their phones.
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use rand::{seq::SliceRandom, Rng};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tower_http::services::ServeDir;

const DUR: u64 = 15_000; // ms per question
const GAP: u64 = 4_000; // a new clue every 4s
const ROUNDS: usize = 10;

type Tx = UnboundedSender<String>;
struct Q { topic: &'static str, clues: [&'static str; 3], opts: [&'static str; 3], ans: usize }
struct Player { name: String, score: u32, gain: u32, pick: Option<usize>, tx: Tx }
struct Room { topic: String, host: Tx, players: BTreeMap<u32, Player>, running: bool, ans: Option<usize>, t0: Instant, next: u32 }
type Rooms = Arc<Mutex<HashMap<String, Room>>>;

macro_rules! q {
    ($t:expr, [$a:expr, $b:expr, $c:expr], [$x:expr, $y:expr, $z:expr], $n:expr) => {
        Q { topic: $t, clues: [$a, $b, $c], opts: [$x, $y, $z], ans: $n }
    };
}

static QS: &[Q] = &[
    // ---- AI ----
    q!("AI", ["🧠", "🕸️", "📉"], ["Decision tree", "Neural network", "Linked list"], 1),
    q!("AI", ["💬", "📚", "🔮"], ["Large language model", "Computer vision", "Clustering"], 0),
    q!("AI", ["🎮", "🏆", "🔁"], ["Supervised learning", "Data mining", "Reinforcement learning"], 2),
    q!("AI", ["👁️", "🖼️", "🔍"], ["Speech recognition", "Computer vision", "Data mining"], 1),
    q!("AI", ["🏷️", "📊", "🎓"], ["Clustering", "Reinforcement learning", "Supervised learning"], 2),
    q!("AI", ["🎨", "🤖", "🖼️"], ["Generative AI", "Search engine", "Antivirus"], 0),
    q!("AI", ["🔥", "🧪", "🐍"], ["TensorFlow", "PyTorch", "Scikit-learn"], 1),
    q!("AI", ["🤗", "😊", "📦"], ["Kaggle", "GitHub", "Hugging Face"], 2),
    q!("AI", ["🎭", "🤥", "🧠"], ["Overfitting", "Hallucination", "Prompting"], 1),
    q!("AI", ["⛰️", "👣", "📉"], ["Gradient descent", "Backpropagation", "Dropout"], 0),
    // ---- Backend dev ----
    q!("Backend dev", ["🦀", "🔒", "⚡"], ["Go", "Rust", "Ruby"], 1),
    q!("Backend dev", ["🐘", "🗄️", "📊"], ["MongoDB", "Redis", "PostgreSQL"], 2),
    q!("Backend dev", ["🐳", "📦", "🚢"], ["Docker", "Kubernetes", "Nginx"], 0),
    q!("Backend dev", ["☕", "🌱", "🍃"], ["Spring Boot", "Laravel", "Express"], 0),
    q!("Backend dev", ["🐹", "💨", "🧵"], ["Rust", "Go", "Kotlin"], 1),
    q!("Backend dev", ["🔴", "💾", "⚡"], ["Memcached", "MongoDB", "Redis"], 2),
    q!("Backend dev", ["🍃", "📄", "🌿"], ["MongoDB", "MySQL", "SQLite"], 0),
    q!("Backend dev", ["☸️", "⚓", "🎛️"], ["Docker", "Kubernetes", "Terraform"], 1),
    q!("Backend dev", ["🔌", "📡", "🔄"], ["REST", "WebSocket", "GraphQL"], 1),
    q!("Backend dev", ["⚖️", "🚦", "🖥️"], ["Load balancer", "Firewall", "Cache"], 0),
    // ---- Security ----
    q!("Security", ["🎣", "📧", "🪝"], ["Ransomware", "Phishing", "Keylogger"], 1),
    q!("Security", ["💉", "🗄️", "⚠️"], ["XSS", "CSRF", "SQL injection"], 2),
    q!("Security", ["🌊", "🖥️", "💥"], ["DDoS", "Phishing", "Brute force"], 0),
    q!("Security", ["🔐", "🌐", "✅"], ["FTP", "HTTPS", "Telnet"], 1),
    q!("Security", ["🔑", "🔨", "🔢"], ["Brute force", "Phishing", "Spoofing"], 0),
    q!("Security", ["🧱", "🔥", "🚫"], ["Antivirus", "VPN", "Firewall"], 2),
    q!("Security", ["🔒", "💰", "💀"], ["Spyware", "Ransomware", "Adware"], 1),
    q!("Security", ["📱", "🔢", "🔑"], ["Two-factor authentication", "Password manager", "Captcha"], 0),
    q!("Security", ["🐴", "🏛️", "💻"], ["Worm", "Rootkit", "Trojan horse"], 2),
    q!("Security", ["⌨️", "👀", "📝"], ["Keylogger", "Cookie", "Botnet"], 0),
    // ---- Graphic design ----
    q!("Graphic design", ["🔴", "🟢", "🔵"], ["CMYK", "RGB", "Grayscale"], 1),
    q!("Graphic design", ["🔤", "🅰️", "✒️"], ["Typography", "Illustration", "Photography"], 0),
    q!("Graphic design", ["📐", "✒️", "♾️"], ["Raster image", "Bitmap", "Vector graphics"], 2),
    q!("Graphic design", ["🖨️", "🩷", "🟨"], ["RGB", "CMYK", "HSL"], 1),
    q!("Graphic design", ["🅿️", "🖼️", "✂️"], ["Photoshop", "Illustrator", "Figma"], 0),
    q!("Graphic design", ["🔲", "📏", "🧱"], ["Gradient", "Kerning", "Grid system"], 2),
    q!("Graphic design", ["🎡", "🌈", "🎨"], ["Color wheel", "Pie chart", "Halftone"], 0),
    q!("Graphic design", ["🟦", "🔍", "🧱"], ["Vector path", "Pixel", "Font"], 1),
    q!("Graphic design", ["⚪", "⚫", "🌗"], ["Contrast", "Opacity", "Saturation"], 0),
    q!("Graphic design", ["⬜", "🌬️", "🧘"], ["Drop shadow", "Texture", "Whitespace"], 2),
    // ---- Motion design ----
    q!("Motion design", ["🔑", "🎞️", "📈"], ["Rotoscoping", "Keyframes", "Rigging"], 1),
    q!("Motion design", ["🐢", "🏃", "📉"], ["Easing", "Looping", "Masking"], 0),
    q!("Motion design", ["🟣", "✨", "🎞️"], ["Premiere Pro", "DaVinci Resolve", "After Effects"], 2),
    q!("Motion design", ["🔁", "🎞️", "♾️"], ["Looping", "Masking", "Parenting"], 0),
    q!("Motion design", ["🔗", "👶", "👨"], ["Rendering", "Parenting", "Tracking"], 1),
    q!("Motion design", ["🎯", "📹", "🔎"], ["Motion tracking", "Color grading", "Keying"], 0),
    q!("Motion design", ["🟩", "🎬", "✂️"], ["Rotoscoping", "Keying", "Warping"], 1),
    q!("Motion design", ["🦴", "🧍", "🔗"], ["Texturing", "Baking", "Rigging"], 2),
    q!("Motion design", ["📚", "🖍️", "🎞️"], ["Storyboard", "Timeline", "Render queue"], 0),
    q!("Motion design", ["🎞️", "⏱️", "2️⃣4️⃣"], ["Aspect ratio", "Frame rate", "Bitrate"], 1),
    // ---- Filmmaking ----
    q!("Filmmaking", ["🌅", "🎥", "🧡"], ["Golden hour", "Jump cut", "Dolly zoom"], 0),
    q!("Filmmaking", ["✂️", "🎞️", "🧩"], ["Lighting", "Editing", "Sound mixing"], 1),
    q!("Filmmaking", ["🎤", "🎣", "🔊"], ["Gimbal", "Tripod", "Boom mic"], 2),
    q!("Filmmaking", ["🎬", "👏", "🔢"], ["Clapperboard", "Light meter", "Teleprompter"], 0),
    q!("Filmmaking", ["🚃", "🎥", "↔️"], ["Pan", "Dolly shot", "Zoom"], 1),
    q!("Filmmaking", ["🔍", "👤", "😮"], ["Wide shot", "Close-up", "Establishing shot"], 1),
    q!("Filmmaking", ["🎨", "🌡️", "🎞️"], ["Color grading", "Foley", "ADR"], 0),
    q!("Filmmaking", ["👣", "🔊", "🥥"], ["Score", "Dubbing", "Foley"], 2),
    q!("Filmmaking", ["🗺️", "🏙️", "🎬"], ["Close-up", "Establishing shot", "Match cut"], 1),
    q!("Filmmaking", ["🤳", "🌊", "🚶"], ["Tripod", "Crane", "Gimbal"], 2),
    // ---- Blender ----
    q!("Blender", ["🐒", "🧊", "🟠"], ["Suzanne", "Utah teapot", "Spot"], 0),
    q!("Blender", ["🍩", "☕", "🎓"], ["Teapot render", "Donut tutorial", "Sculpt mode"], 1),
    q!("Blender", ["🚴", "🔦", "📸"], ["Eevee", "Workbench", "Cycles"], 2),
    q!("Blender", ["🦊", "⚡", "🕐"], ["Cycles", "Eevee", "Workbench"], 1),
    q!("Blender", ["🗿", "🖐️", "🖌️"], ["Edit mode", "Sculpt mode", "Pose mode"], 1),
    q!("Blender", ["🔧", "📚", "🔁"], ["Modifier stack", "Node tree", "Timeline"], 0),
    q!("Blender", ["🔢", "🧶", "🔗"], ["Shader editor", "Geometry nodes", "Compositor"], 1),
    q!("Blender", ["🦴", "🧍", "🎛️"], ["Armature", "Particle system", "UV map"], 0),
    q!("Blender", ["🗺️", "🧵", "✂️"], ["Retopology", "UV unwrapping", "Baking"], 1),
    q!("Blender", ["✏️", "🧊", "🎞️"], ["Grease Pencil", "Compositor", "Video editor"], 0),
];

fn send(tx: &Tx, v: Value) { let _ = tx.send(v.to_string()); }

fn cast(r: &Room, v: Value) {
    let s = v.to_string();
    let _ = r.host.send(s.clone());
    for p in r.players.values() { let _ = p.tx.send(s.clone()); }
}

fn lobby(r: &Room, code: &str) -> Value {
    json!({"t":"lobby","code":code,"topic":r.topic,"players":r.players.values().map(|p| p.name.clone()).collect::<Vec<_>>()})
}

fn scores(r: &Room) -> Value {
    let mut v: Vec<_> = r.players.values().map(|p| (p.name.as_str(), p.score, p.gain)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    Value::Array(v.into_iter().map(|(n, s, g)| json!({"n": n, "s": s, "g": g})).collect())
}

async fn ws_handler(ws: WebSocketUpgrade, State(r): State<Rooms>) -> impl IntoResponse {
    ws.on_upgrade(move |s| conn(s, r))
}

async fn conn(sock: WebSocket, rooms: Rooms) {
    let (mut sink, mut stream) = sock.split();
    let (tx, mut rx) = unbounded_channel::<String>();
    tokio::spawn(async move {
        while let Some(m) = rx.recv().await {
            if sink.send(Message::Text(m)).await.is_err() { break; }
        }
    });
    let mut me: Option<(String, Option<u32>)> = None; // (room code, player id; None = host)

    while let Some(Ok(m)) = stream.next().await {
        let t = match m { Message::Text(t) => t, Message::Close(_) => break, _ => continue };
        let Ok(v) = serde_json::from_str::<Value>(&t) else { continue };
        match v["t"].as_str().unwrap_or("") {
            "host" => {
                let topic = v["topic"].as_str().unwrap_or("").to_string();
                if !QS.iter().any(|q| q.topic == topic) {
                    send(&tx, json!({"t":"err","msg":"Pick a subject first."}));
                    continue;
                }
                let code = loop {
                    let c = rand::thread_rng().gen_range(1000..10000).to_string();
                    if !rooms.lock().unwrap().contains_key(&c) { break c; }
                };
                let mut g = rooms.lock().unwrap();
                g.insert(code.clone(), Room { topic: topic.clone(), host: tx.clone(), players: BTreeMap::new(), running: false, ans: None, t0: Instant::now(), next: 1 });
                send(&tx, json!({"t":"lobby","code":code,"topic":topic,"players":[]}));
                me = Some((code, None));
            }
            "join" => {
                let code = v["code"].as_str().unwrap_or("").trim().to_string();
                let name: String = v["name"].as_str().unwrap_or("").trim().chars().take(14).collect();
                if name.is_empty() { continue; }
                let mut g = rooms.lock().unwrap();
                match g.get_mut(&code) {
                    None => send(&tx, json!({"t":"err","msg":"Room not found. Check the code."})),
                    Some(r) if r.running => send(&tx, json!({"t":"err","msg":"This game already started."})),
                    Some(r) => {
                        let id = r.next;
                        r.next += 1;
                        r.players.insert(id, Player { name, score: 0, gain: 0, pick: None, tx: tx.clone() });
                        let l = lobby(r, &code);
                        cast(r, l);
                        me = Some((code, Some(id)));
                    }
                }
            }
            "start" => {
                if let Some((code, None)) = &me {
                    let mut g = rooms.lock().unwrap();
                    if let Some(r) = g.get_mut(code) {
                        if !r.running && !r.players.is_empty() {
                            r.running = true;
                            for p in r.players.values_mut() { p.score = 0; }
                            tokio::spawn(game(rooms.clone(), code.clone()));
                        }
                    }
                }
            }
            "pick" => {
                if let Some((code, Some(id))) = &me {
                    let c = v["c"].as_u64().unwrap_or(9) as usize;
                    let mut g = rooms.lock().unwrap();
                    if let Some(r) = g.get_mut(code) {
                        if let (Some(a), true) = (r.ans, c < 3) {
                            let el = r.t0.elapsed().as_millis() as u64;
                            let mut done = false;
                            if let Some(p) = r.players.get_mut(id) {
                                if p.pick.is_none() && el <= DUR + 400 {
                                    p.pick = Some(c);
                                    // reflex scoring: 400 base + up to 600 for speed
                                    p.gain = if c == a { (400 + 600 * DUR.saturating_sub(el) / DUR) as u32 } else { 0 };
                                    send(&p.tx, json!({"t":"locked"}));
                                    done = true;
                                }
                            }
                            if done {
                                let n = r.players.values().filter(|p| p.pick.is_some()).count();
                                send(&r.host, json!({"t":"count","n":n,"total":r.players.len()}));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if let Some((code, id)) = me {
        let mut g = rooms.lock().unwrap();
        match id {
            None => {
                if let Some(r) = g.remove(&code) { cast(&r, json!({"t":"err","msg":"The host left the game."})); }
            }
            Some(id) => {
                if let Some(r) = g.get_mut(&code) {
                    r.players.remove(&id);
                    if !r.running { let l = lobby(r, &code); cast(r, l); }
                }
            }
        }
    }
}

async fn game(rooms: Rooms, code: String) {
    let topic = match rooms.lock().unwrap().get(&code) { Some(r) => r.topic.clone(), None => return };
    // one subject per room: never mix topics
    let mut qs: Vec<&'static Q> = QS.iter().filter(|q| q.topic == topic).collect();
    qs.shuffle(&mut rand::thread_rng());
    qs.truncate(ROUNDS);
    let n = qs.len();

    for (i, q) in qs.into_iter().enumerate() {
        {
            let mut g = rooms.lock().unwrap();
            let Some(r) = g.get_mut(&code) else { return };
            r.ans = Some(q.ans);
            r.t0 = Instant::now();
            for p in r.players.values_mut() { p.pick = None; p.gain = 0; }
            let s = scores(r);
            cast(r, json!({"t":"q","i":i+1,"n":n,"topic":q.topic,"opts":q.opts,"dur":DUR,"scores":s}));
        }
        let (mut k, mut ms) = (0usize, 0u64);
        loop {
            {
                let g = rooms.lock().unwrap();
                let Some(r) = g.get(&code) else { return };
                while k < 3 && ms >= k as u64 * GAP {
                    cast(r, json!({"t":"clue","idx":k,"e":q.clues[k]}));
                    k += 1;
                }
                if ms >= DUR || (!r.players.is_empty() && r.players.values().all(|p| p.pick.is_some())) { break; }
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
            ms += 250;
        }
        {
            let mut g = rooms.lock().unwrap();
            let Some(r) = g.get_mut(&code) else { return };
            r.ans = None;
            let mut counts = [0u32; 3];
            for p in r.players.values_mut() {
                if let Some(c) = p.pick { counts[c] += 1; }
                if p.pick == Some(q.ans) { p.score += p.gain } else { p.gain = 0 }
            }
            let base = json!({"t":"reveal","answer":q.ans,"counts":counts,"scores":scores(r)});
            send(&r.host, base.clone());
            for p in r.players.values() {
                let mut m = base.clone();
                m["you"] = json!({"ok": p.pick == Some(q.ans), "gain": p.gain, "score": p.score, "none": p.pick.is_none()});
                send(&p.tx, m);
            }
        }
        tokio::time::sleep(Duration::from_millis(5500)).await;
    }

    let mut g = rooms.lock().unwrap();
    if let Some(r) = g.get_mut(&code) {
        r.running = false;
        let s = scores(r);
        cast(r, json!({"t":"end","scores":s}));
    }
}

#[tokio::main]
async fn main() {
    let rooms: Rooms = Arc::default();
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback_service(ServeDir::new("static"))
        .with_state(rooms);
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let l = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    println!("SkillBoost Reflex Arena running on http://localhost:{port}");
    axum::serve(l, app).await.unwrap();
}
