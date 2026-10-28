# Demo

Start `just mock` for nine canned client fixtures, or `just run` for persisted behavior scoring. In another terminal run `just demo-seed`. Output is captured in `target/logs/demo-seed.log`; Luna reports the summary.

Real mode demonstrates scenario 1 (80 friendly messages, review before block then high), scenario 2 (ten senders in one minute, high and feed protection), and scenario 7 (reciprocal high-volume couple and long answered calls, low). Each run creates unique identities to avoid history contamination. Fixtures are synthetic; scoring uses real rules and SQLite rather than sender-name overrides.

Luna reports the latest real-mode run returned scenario 1 high/85, scenario 2 high/73, and scenario 7 low/0. These synthetic examples are not accuracy measurements.

Mock mode exercises all nine expected tiers and marks every response simulated. Media, slang updates and evidence export are canned client fixtures; they do not demonstrate working modality inference, moderator training or a verifiable evidence export. Full nine-scenario real integration awaits teammate modules.
