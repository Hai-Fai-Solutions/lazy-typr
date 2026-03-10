# Changelog

All notable changes to this project will be documented in this file.
This file is automatically maintained by [cocogitto](https://github.com/cocogitto/cocogitto).

- - -
## [v0.6.0](https://github.com/Hai-Fai-Solutions/lazy-typr/compare/0370f37a77313c0cd4e8d16da778b099ea98234f..v0.6.0) - 2026-03-10
#### Features
- transcribe-by-default + CI paths allowlist (#20) - ([0370f37](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/0370f37a77313c0cd4e8d16da778b099ea98234f)) - Christian Polzer

- - -

## [v0.5.1](https://github.com/Hai-Fai-Solutions/lazy-typr/compare/f1522c3f0f0d42b9b8ddf6033d396a9f885da9be..v0.5.1) - 2026-03-08
#### Bug Fixes
- preserve config language unless --language is provided (#19) - ([f1522c3](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/f1522c3f0f0d42b9b8ddf6033d396a9f885da9be)) - Christian Polzer

- - -

## [v0.5.0](https://github.com/Hai-Fai-Solutions/lazy-typr/compare/3033784ec53600a60a12a81a722ff5bec30830e1..v0.5.0) - 2026-03-08
#### Features
- **(audio)** add WebRTC VAD as two-stage noise gate (#18) - ([3033784](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/3033784ec53600a60a12a81a722ff5bec30830e1)) - Christian Polzer

- - -

## [v0.4.0](https://github.com/Hai-Fai-Solutions/lazy-typr/compare/bf66c772ac79628b1bd1adb57715f5f2436be212..v0.4.0) - 2026-03-08
#### Features
- ydotool backend for compositor-agnostic text injection (KDE support) (#17) - ([bf66c77](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/bf66c772ac79628b1bd1adb57715f5f2436be212)) - Christian Polzer

- - -

## [v0.3.1](https://github.com/Hai-Fai-Solutions/lazy-typr/compare/61bccb127da463518289b5d38138c000a66dc0c1..v0.3.1) - 2026-03-08
#### Bug Fixes
- add Vulkan dependencies to CI and release workflows (#16) - ([61bccb1](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/61bccb127da463518289b5d38138c000a66dc0c1)) - Christian Polzer

- - -

## [v0.3.0](https://github.com/Hai-Fai-Solutions/lazy-typr/compare/4ab592d4332fe56544af685c445b0032d09c11e2..v0.3.0) - 2026-03-08
#### Continuous Integration
- simplify branch model — remove develop, add bugfix/** (#10) - ([3e58aef](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/3e58aefcd76534b9dd23c45ea7db299bc50d2455)) - Christian Polzer
- prevent duplicate runs when push and PR fire for same commit (#8) - ([4ab592d](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/4ab592d4332fe56544af685c445b0032d09c11e2)) - Christian Polzer
#### Features
- GPU inference via Vulkan (runtime opt-in) (#15) - ([47d7f90](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/47d7f901656c080041115dd3f512c4cbd37959e0)) - Christian Polzer
#### Miscellaneous Chores
- add pre-commit auto-formatting and VSCode DX (#11) - ([b3a1b4d](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/b3a1b4d094e48b8087e1bc32a12f885989b08be9)) - Christian Polzer

- - -

## [v0.2.0](https://github.com/Hai-Fai-Solutions/lazy-typr/compare/650c80baaf61a837822d546e205e86c11a1ff640..v0.2.0) - 2026-03-07
#### Features
- add Renovate Bot for automated dependency updates (#7) (#9) - ([650c80b](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/650c80baaf61a837822d546e205e86c11a1ff640)) - Christian Polzer

- - -

## [v0.1.1](https://github.com/Hai-Fai-Solutions/lazy-typr/compare/d395d2321366efd5052b6fedd1298f0f58b35e1f..v0.1.1) - 2026-03-07
#### Bug Fixes
- handle nothing-to-bump gracefully in CI - ([d395d23](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/d395d2321366efd5052b6fedd1298f0f58b35e1f)) - christian polzer

- - -

## [v0.1.0](https://github.com/Hai-Fai-Solutions/lazy-typr/compare/v0.0.4..v0.1.0) - 2026-03-07
#### Bug Fixes
- simplify push step — always push follow-tags - ([821f410](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/821f410e89835a592d51df5d7d89c06258272d72)) - christian polzer
- use cocogitto separator in CHANGELOG - ([b4e906b](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/b4e906b05c7b2a5d088669a2d2c448dfc56227ed)) - christian polzer
- disable cog check and scope bump to latest tag - ([bdbee68](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/bdbee68636e0c5c962504e8daf6fd1c27a6cef79)) - christian polzer
- replace bump_commit_message with skip_ci in cog.toml - ([59de3b4](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/59de3b4ed525d9be6038ed32c85759f5931cc709)) - christian polzer
#### Continuous Integration
- add workflow_dispatch trigger to bump workflow - ([a6f8906](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/a6f8906b483d8c70224fdcd6cc7dd32f5a9f3796)) - christian polzer
#### Features
- introduce cocogitto for automated semver bumping (#6) - ([aedddab](https://github.com/Hai-Fai-Solutions/lazy-typr/commit/aedddabb0c308eb7433555e48ec8ff0ece35de54)) - Christian Polzer

- - -

