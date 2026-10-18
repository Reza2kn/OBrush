# REA investigation workflow for OBrush

REA is a research tool for gathering evidence; it does not supply ZBrush's feature set or establish parity by itself. OBrush's implementation and comparisons follow the [feature contract](FEATURE_PARITY.md).

## Built locally from source

At the user's request, [morluto/rea](https://github.com/morluto/rea) was cloned and built inside this workspace on **2026-10-12**:

| Item | Recorded value |
| --- | --- |
| Checkout | `/Users/Ajab/AI/OBrush/.tools/rea` |
| Source revision | [`ec2d251c0ad8382e0665546be5ec8f5f1c175962`](https://github.com/morluto/rea/tree/ec2d251c0ad8382e0665546be5ec8f5f1c175962) |
| Source package / CLI version | `rea-agents` / `6.4.0` |
| Existing build runtime | Node `22.22.3`, npm `10.9.8` |
| Source development pin | Node `24.18.0`, npm `11.16.0`; the existing Node satisfies the declared supported range |
| Dependency installation | `npm ci` succeeded: 400 packages installed; npm's audit reported zero vulnerabilities at that time |
| Runtime build | `npm run build:cached` succeeded: 3 tasks; bundler emitted two nonfatal dynamic-import warnings |
| CLI smoke checks | `--version`, `--help`, `inspect-macho --help`, `inspect-artifact --help`, `providers --json` succeeded |
| Ghidra readiness | After local toolchain installation, `doctor --provider ghidra --json` exited `0`, `healthy: true` |
| Deep analysis providers | Locally configured Ghidra 12.1.4; Hopper executable missing; IDA MCP unconfigured |
| Client registration | No global MCP/skill registration was performed; invoke the local CLI directly |
| ZBrush target | Subsequently installed by the user; local bundle metadata confirms 2026.2.1; static metadata/targeted label inspection completed |

The git revision pins this **source checkout**. Its package version string alone does not identify the same functionality as the published npm release. The CLI's provider report returned `build_commit: null`; the revision above was independently observed with `git rev-parse HEAD`.

Initial readiness outputs are retained locally at `.tools/rea-evidence/providers.json` and `.tools/rea-evidence/doctor-ghidra.json`; those initial missing-engine results were superseded by `.tools/rea-evidence/doctor-ghidra-ready.json`. Raw outputs stay in excluded local tooling storage. CLI startup and advertised capabilities do not establish a successful real engine session.

## Reproduce the local source build

The following reproduces the local source build from the OBrush workspace root, including an explicit checkout of the observed revision. The completed run cloned that revision from main, then ran `npm ci` and `npm run build:cached`. Inspect the pinned repository instructions and package scripts before running a future revision.

```sh
git clone --depth 1 https://github.com/morluto/rea.git .tools/rea
git -C .tools/rea fetch --depth 1 origin ec2d251c0ad8382e0665546be5ec8f5f1c175962
git -C .tools/rea checkout --detach ec2d251c0ad8382e0665546be5ec8f5f1c175962
npm --prefix .tools/rea ci
npm --prefix .tools/rea run build:cached
node .tools/rea/scripts/rea.mjs --version
node .tools/rea/scripts/rea.mjs providers --json
node .tools/rea/scripts/rea.mjs doctor --provider ghidra --json
```

The final command needs the local Ghidra/JDK environment shown below. This recipe creates only the local checkout/dependencies/build outputs; it does not run REA's agent setup. REA's npm prepare hook configures hooks inside its own checkout. No system toolchain upgrade, Hopper installation, agent configuration change or app restart was needed for the source build.

The installed runtime meets the [package's declared engine range](https://github.com/morluto/rea/blob/ec2d251c0ad8382e0665546be5ec8f5f1c175962/package.json). Source build instructions are in [REA's AGENTS.md](https://github.com/morluto/rea/blob/ec2d251c0ad8382e0665546be5ec8f5f1c175962/AGENTS.md) and [CONTRIBUTING.md](https://github.com/morluto/rea/blob/ec2d251c0ad8382e0665546be5ec8f5f1c175962/CONTRIBUTING.md).

## Available investigations now

| Evidence route | Current use | Limit |
| --- | --- | --- |
| Official Maxon help and release descriptions | Build feature inventory and independently implement documented workflows | Establishes documentation, not measured tool response or internal algorithms |
| REA built-in artifact / macOS inspection | Inspect a selected OBrush build's container, architecture, imports and file identity | Metadata does not decompile sculpting algorithms or establish parity |
| REA browser capture | Later record OBrush browser scenarios with a declared browser and output directory | Requires a successful real browser operation; no ZBrush desktop reference is available through a website |
| REA native engine analysis | Investigate focused questions against the installed ZBrush ARM64 slice with explicit resource limits | Scoped readiness passes; a real query must finish successfully before engine findings can be claimed |

Static inventory has been performed on both executables. For subsequent OBrush builds, select the actual output binary and refresh its evidence rather than reusing an earlier hash:

The final OBrush record was refreshed against `.artifacts/OBrush.app/Contents/MacOS/OBrush`: bundle version **0.1.0**, **5,922,544 bytes**, ARM64 Mach-O, SHA-256 **`d1016cb942106fbb52c3b85786fc410494544d3783cd4bfdef020a7d75ed0f0f`**. This is the executable inside the **ad-hoc signed macOS app**. The raw Cargo executable at `target/release/obrush` is **5,938,944 bytes**, SHA-256 **`7414f7dcd720a33b0cf77e984261afcfdfd78a06490daba909ce3e482b96a057`**; it has different bytes because packaging uses a fresh-inode copy and signs the complete app. Both REA operations succeeded on the final signed bundle, and `codesign --verify --deep --strict .artifacts/OBrush.app` passed. This supersedes earlier build/signature records. Release mode describes this local optimized build, without asserting published distribution or feature parity.

```sh
node .tools/rea/scripts/rea.mjs inspect-macho /absolute/path/to/obrush-binary --json
node .tools/rea/scripts/rea.mjs inspect-artifact /absolute/path/to/obrush-binary --json
```

Retain complete JSON locally and summarize its facts, failures and coverage in the evidence record. The delivered records are [ZBrush native inspection](evidence/zbrush-native-inspection.json), [OBrush native inspection](evidence/obrush-native-inspection.json) and [bounded brush-label observations](evidence/zbrush-brush-label-observations.json). The last record reads only the ARM64 `__TEXT,__cstring` section at coordinates returned by REA; it retains short identifiers/UI labels and omits diagnostic source paths. Labels such as `ClayBuildup`, `DamStandard` and `Tablet Pressure` are observed bytes, not recovered brush algorithms or verified behavior. The selected ZBrush export inventory exposes no brush-function symbols; its incomplete/stripped inventory cannot establish their absence from the binary. For a browser build, check the actual command's `--help` and required browser support before recording a bounded sculpt/undo/save scenario. REA's [CLI guide](https://github.com/morluto/rea/blob/ec2d251c0ad8382e0665546be5ec8f5f1c175962/docs/cli.md) defines the command/result contracts.

One bounded Ghidra query searched the exact ARM64 slice for `ClayBuildup`. It returned **`provider_timeout`**, exit `1`, after **123.81 seconds** with a 120,000 ms startup deadline and 2 GiB JVM heap. Initial import/analysis did not finish, so it returned **no engine-derived findings or native call relationships**. No full-binary retry or longer deadline was attempted. See the [query result and resource limits](evidence/zbrush-ghidra-brush-query.json). Scoped toolchain readiness remains successful, documented in [toolchain provenance](evidence/rea-ghidra-toolchain.json); readiness and completed large-target analysis are distinct.

## Native prerequisites and later local invocation

Native decompilation needs Hopper, Ghidra or an IDA adapter. This checkout's [installation guide](https://github.com/morluto/rea/blob/ec2d251c0ad8382e0665546be5ec8f5f1c175962/docs/installation.md) supports existing Ghidra **12.1.x** with a compatible full 64-bit JDK and host native decompiler. The pre-existing `/usr/bin/java` was only a launcher with no usable JDK. The user-authorized local setup downloaded official [Ghidra 12.1.4](https://github.com/NationalSecurityAgency/ghidra/releases/tag/Ghidra_12.1.4_build) and [Temurin JDK 21.0.12.1+1 for macOS ARM64](https://github.com/adoptium/temurin21-binaries/releases/tag/jdk-21.0.12.1%2B1), verifying their vendor SHA-256 digests before extraction. Combined download size was 769,723,002 bytes. Ghidra's release ZIP includes decompiler C++ source but lacks the macOS executable; that component was built locally with existing `clang`/`make`/`bison`, using `make -j4 ghidra_opt ARCH_TYPE='-arch arm64' ADDITIONAL_FLAGS='-mmacosx-version-min=11.0 -w'`, then placed at `Ghidra/Features/Decompiler/os/mac_arm_64/decompile`. No system JDK or global application installation was changed. Hopper and IDA remain unused.

Use per-command configuration and explicit provider selection. The local Ghidra/JDK directories are `.tools/ghidra/ghidra_12.1.4_PUBLIC` and `.tools/jdk/jdk-21.0.12.1+1/Contents/Home`; substitute their absolute workspace paths below. REA readiness verified both versions and the ARM64 native decompiler. Resource controls for the first ZBrush query were `REA_GHIDRA_STARTUP_TIMEOUT_MS=120000` and `GHIDRA_HEADLESS_MAXMEM=2G`; these bound startup waiting and JVM heap, not total process-family memory. The universal executable was reduced to its exact ARM64 slice with `lipo -thin arm64`, and the slice hash was checked against REA's artifact inventory. No commercial executable bytes were patched.

```sh
GHIDRA_INSTALL_DIR=/absolute/path/to/ghidra_12.1.4_PUBLIC \
  node .tools/rea/scripts/rea.mjs doctor --provider ghidra --json

GHIDRA_INSTALL_DIR=/absolute/path/to/ghidra_12.1.4_PUBLIC \
  node .tools/rea/scripts/rea.mjs analyze /absolute/path/to/target --provider ghidra --json

GHIDRA_INSTALL_DIR=/absolute/path/to/ghidra_12.1.4_PUBLIC \
  node .tools/rea/scripts/rea.mjs search /absolute/path/to/target "focused feature term" --provider ghidra --json
```

Prefer a documented public workflow or input/output comparison whenever it answers the question. A string match is a lead, not proof of the feature's implementation. Follow relevant cross-references and function evidence only for the scoped question. Persist snapshots tied to target bytes, provider and parameters; a cached result does not prove the current application behaves identically. Observe session completion and cleanup before starting another heavy query. These lifecycle semantics are documented in the [native CLI and snapshot guide](https://github.com/morluto/rea/blob/ec2d251c0ad8382e0665546be5ec8f5f1c175962/docs/cli.md).

## Comparison protocol

1. Select one feature ID and a specific uncertainty, such as pressure-to-displacement response, mask interpolation or a file field. Pin application versions, OBrush commit and fixture hashes.
2. Start with official documentation and independently authored test meshes/strokes. Write a behavior specification and numerical acceptance tolerance before comparing results.
3. When ZBrush is available, record its exact settings, input events and exported result. Run the same operation in OBrush; compare geometry, attributes, topology, persistence, latency and visible interaction as applicable.
4. Use REA only to resolve a remaining evidence gap. Inspect the selected executable's identity/architecture, choose a provider, validate real readiness and retain findings with provenance and acknowledged unknowns.
5. Implement the specification in Rust from independently written code. Keep commercial binaries, decompiled proprietary source, copied artwork and raw commercial implementation dumps out of the OBrush repository.
6. Promote a feature only when its full acceptance gate passes; preserve mismatches and unsupported parameters in the parity matrix. Native format findings require versioned fixtures and independent read/write verification, not a guessed ZPR/ZTL parser.

Each report distinguishes **documented behavior**, **observed reference behavior**, **binary observations**, **inference**, **OBrush tests** and **unknowns**. Store target hashes, commands, selected provider/version, parameters, input/output artifacts, coverage/truncation, runtime/resource measurements and cleanup outcome. Redact credentials rather than recording ambient environments.

The installed ZBrush target enables focused investigation. Static executable identity, labels and dependencies still do not establish a reference behavior audit. A separate [real OBJ interchange comparison](evidence/obj-zbrush-roundtrip.json) records one OBrush-browser export imported into ZBrush 2026.2.1 and exported back: matching vertex/face counts and indices with maximum position error `4.690415757354602e-8`. It exposed ZBrush's `#MRGB` vertex-color representation and an OBrush import gap. The corrected browser importer now re-exports all 10,242 RGB entries; the decoded first RGB matches `[165,175,188] / 255` within `1.6470588226624727e-8`, while mask semantics remain unsupported. This exploratory exchange uses uniform default color and no preregistered tolerance; it is measured evidence for one mesh, without establishing matched sculpt-stroke behavior or proprietary-format compatibility. Broader local validation is recorded in [VALIDATION.md](VALIDATION.md).
