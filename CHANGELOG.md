## [0.1.17] - 2026-10-20

### 🚀 Features

- Support Jenkins 2.580.1 LTS

### 🚜 Refactor

- [**breaking**] Migrate to reqx

### ⚙️ Miscellaneous Tasks

- Polish ci and tests
- Release jenkins-sdk version 0.1.17
## [0.1.16] - 2026-01-13

### 🐛 Bug Fixes

- Ci

### 🚜 Refactor

- [**breaking**] Restructure crate; add blocking+TLS features; expand Jenkins core APIs

### ⚙️ Miscellaneous Tasks

- *(docs)* Update README.md
- Update Cargo.toml
- Release jenkins-sdk version 0.1.16
## [0.1.15] - 2025-12-29

### 🚜 Refactor

- *(core)* Tighten URL and error handling

### ⚙️ Miscellaneous Tasks

- *(ci)* Update
- Release jenkins-sdk version 0.1.15
## [0.1.14] - 2025-12-09

### ⚙️ Miscellaneous Tasks

- Release jenkins-sdk version 0.1.13
- Release jenkins-sdk version 0.1.14
## [0.1.12] - 2025-12-09

### 🐛 Bug Fixes

- Normalize Jenkins base URLs

### ⚙️ Miscellaneous Tasks

- Release jenkins-sdk version 0.1.12
## [0.1.11] - 2025-09-19

### 🐛 Bug Fixes

- Align middleware exports and crumb timeouts
- *(retry)* Correct exponential backoff timings

### 📚 Documentation

- Update usage samples for fallible builder chain

### 🚜 Refactor

- *(endpoint)* Add parsing hook and robust param encoding
- *(client)* Make builders fallible and delegate decoding to endpoints

### 🧪 Testing

- Add wiremock helpers and crumb/error flows

### ⚙️ Miscellaneous Tasks

- Add .editorconfig
- Add .gitattributes
- Update description
- Release jenkins-sdk version 0.1.11
## [0.1.10] - 2025-06-05

### ⚙️ Miscellaneous Tasks

- Update .gitignore
- *(docs)* Update README
- Release jenkins-sdk version 0.1.10
## [0.1.9] - 2025-06-05

### 🚀 Features

- Add new endpoints for fetching the job details,the last build information,the console log of the last build
- Add JobDetail, LastBuildInfo & LastBuildConsole endpoints

### ⚙️ Miscellaneous Tasks

- Release jenkins-sdk version 0.1.9
## [0.1.8] - 2025-05-26

### 🚀 Features

- Switch to exponential back-off
- Add method/url context
- Add StopBuild

### ⚙️ Miscellaneous Tasks

- Release jenkins-sdk version 0.1.8
## [0.1.7] - 2025-05-25

### 🐛 Bug Fixes

- Ci

### ⚙️ Miscellaneous Tasks

- Release jenkins-sdk version 0.1.7
## [0.1.6] - 2025-05-25

### 🚀 Features

- Add middleware

### 🐛 Bug Fixes

- Ci

### ⚙️ Miscellaneous Tasks

- Update docs
- Release jenkins-sdk version 0.1.6
## [0.1.5] - 2025-03-29

### 🚀 Features

- Redesign SDK with structured endpoints, sync/async support, and improved Rust idiomatic API

### ⚙️ Miscellaneous Tasks

- Release jenkins-sdk version 0.1.5
## [0.1.4] - 2025-03-11

### 🐛 Bug Fixes

- BuildWithParameters

### ⚙️ Miscellaneous Tasks

- *(docs)* Update README.md
- Release jenkins-sdk version 0.1.4
## [0.1.3] - 2025-03-01

### ⚙️ Miscellaneous Tasks

- Disable reqwest default features
- Release jenkins-sdk version 0.1.3
## [0.1.2] - 2025-02-28

### ⚙️ Miscellaneous Tasks

- Add CHANGELOG.md
- Release jenkins-sdk version 0.1.2
## [0.1.1] - 2025-02-28

### ⚙️ Miscellaneous Tasks

- Init commit
- Release jenkins-sdk version 0.1.1
