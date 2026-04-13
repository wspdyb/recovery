# תהליך עבודה ב-GitHub

גרסה: 1.0  
תאריך: 12 באפריל 2026  
שפת עבודה: עברית

## מבוא

מסמך זה מתאר את תהליך העבודה ב-GitHub לפרויקט PhoenixRecover. הוא מבוסס על Git Flow עם התאמות לפרויקט זה.

## Branches

### Main Branches

- **main**: קוד יציב, ready for release
- **develop**: קוד בפיתוח, integration branch

### Supporting Branches

- **feature/***: פיתוח תכונות חדשות
  - feature/acquisition-layer
  - feature/ntfs-parser

- **fix/***: תיקון באגים
  - fix/carving-crash
  - fix/ui-freeze

- **research/***: ניסויים ו-R&D
  - research/ai-reconstruction
  - research/plugin-system

## Workflow

### 1. התחלת עבודה על feature

```bash
# checkout develop
git checkout develop
git pull origin develop

# create feature branch
git checkout -b feature/your-feature-name

# work on feature
# commit regularly
git add .
git commit -m "feat: add acquisition layer"
```

### 2. פיתוח

- כתוב tests לפני/אחרי קוד
- run `cargo check` ו-`cargo test` לפני commit
- commit messages: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`
- push regularly ל-branch

### 3. Pull Request

כאשר ה-feature מוכן:

```bash
git push origin feature/your-feature-name
```

צור PR ב-GitHub עם:
- Title: [FEATURE] Add acquisition layer
- Description:
  - מה נוסף/שונה
  - screenshots אם UI
  - breaking changes אם יש
  - migration impact
  - test status
  - known risks

### 4. Code Review

- לפחות reviewer אחד
- CI חייב לעבור
- fix כל הערות
- squash commits אם צריך

### 5. Merge

- Merge ל-develop דרך GitHub
- Delete branch אחרי merge

### 6. Release

כאשר develop stable:
- Create release branch: `release/v1.0.0`
- Test thoroughly
- Merge ל-main
- Tag release
- Deploy

## Labels

### Priority
- priority/critical
- priority/high
- priority/medium
- priority/low

### Type
- type/feature
- type/bug
- type/docs
- type/refactor
- type/test

### Module
- core-io
- parser
- carving
- validation
- fusion
- scoring
- ui
- db
- docs
- security
- performance
- forensic

### Status
- status/ready-for-review
- status/in-progress
- status/blocked
- status/wont-fix

## Issues

### Templates

#### Feature Request
- Title: [FEATURE] Add support for APFS
- Description: detailed requirements
- Acceptance criteria
- Dependencies

#### Bug Report
- Title: [BUG] Carving crashes on large files
- Steps to reproduce
- Expected behavior
- Actual behavior
- Environment (OS, version)
- Logs/screenshots

### Assignment
- Assign למי שעובד על זה
- Update status ב-labels

## Projects

### Boards

#### MVP Core
- To Do
- In Progress
- Done

#### Recovery Quality
- Planned
- In Progress
- Completed

#### Forensic Expansion
- Backlog
- Ready
- Doing
- Done

### Milestones
- Foundation v0.1
- MVP Core v1.0
- Recovery Quality v1.5
- Forensic v2.0

## CI/CD

### GitHub Actions

#### On Push
- Checkout
- Setup Rust
- Cache dependencies
- Run `cargo check`
- Run `cargo test`
- Run `cargo clippy`
- Run `cargo fmt --check`

#### On PR
- Additional: build release
- Test UI build

#### On Release
- Build binaries for Windows/Linux/macOS
- Create GitHub release
- Upload artifacts

## Code Quality

### Rust
- `cargo fmt` לפני commit
- `cargo clippy` ללא warnings
- Tests coverage >80%
- No unsafe code ללא הצדקה

### UI
- ESLint ללא errors
- TypeScript strict mode
- Responsive design
- RTL support

### Docs
- README updated
- Code comments in English
- API docs generated

## Communication

### Standups
- Daily updates ב-issues
- Blockers reported immediately

### Reviews
- Constructive feedback
- Approve with comments
- Request changes if needed

### Releases
- Changelog generated
- Release notes detailed
- Beta releases ל-testing

## Security

- No secrets in code
- Dependency scanning
- Security audits before release
- Vulnerability disclosure process

## Best Practices

- One feature per PR
- Small, focused commits
- Rebase instead of merge
- Keep branches short-lived
- Use draft PRs for WIP
- Tag reviewers appropriately
- Close issues when done
- Reference issues in commits