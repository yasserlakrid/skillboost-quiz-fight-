# SkillBoost Reflex Arena

Vanilla JS front end + Rust (axum, WebSocket) back end.

## Run
    cargo run --release          # needs Rust >= 1.85
    # open http://localhost:3000   (set PORT=xxxx to change)

- Big screen / projector: open the site, click **Host a game**. It shows the room code, the topic, the emoji clues and the leaderboard.
- Players: open the same address on their phones (same Wi-Fi, use the host machine's LAN IP), enter the code and a name.

## How a round works
- The host picks ONE subject per room (AI, Backend dev, Security, Graphic design, Motion design, Filmmaking or Blender). Subjects are never mixed. Each subject has 10 questions, all played in random order.
- The topic is shown at the top. Three emoji clues are revealed one by one (0s, 4s, 8s) in a 15s round.
- Players pick 1 of 3 answers. Correct = 400 points + up to 600 for speed.
- The round ends early when everyone has answered.

## Add questions
Edit the `QS` list in `src/main.rs`:
    q!("Blender", ["🐒","🧊","🟠"], ["Suzanne","Utah teapot","Spot"], 0),   // last number = index of the right answer
# skillboost-quiz-fight-
