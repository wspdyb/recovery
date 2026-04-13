# PhoenixRecover / Recovery Platform — תוכנית עבודה מאוחדת, משודרגת ומעשית

גרסה: 2.0  
תאריך: 12 באפריל 2026  
שפת עבודה: עברית  
סטטוס: מסמך עבודה ראשי לפרויקט

---

## 1. מטרת המסמך

זהו מסמך העבודה הראשי והמאוחד של הפרויקט. הוא משלב בין:
- מסמך PhoenixRecover
- מסמך התכנון והשלד שכבר נבנו לפרויקט
- ההחלטות שעלו מההשוואה ביניהם
- דרישתך לבנות מערכת שלמה, מודולרית, ניתנת לסנכרון ב־GitHub, ומבוססת על חלוקת עבודה בפועל

המטרה של המסמך היא לא רק לתאר חזון, אלא להגדיר:
- מה בדיוק בונים
- באיזה סדר בונים
- באילו שפות וכלים משתמשים
- איך מחלקים את העבודה
- איך מסנכרנים דרך GitHub
- איך שומרים תוצאות סריקה
- איך מתחילים שלב־שלב בלי להיתקע

---

## 2. יעד העל של המערכת

לבנות פלטפורמת שחזור קבצים מקצועית, היברידית, מודולרית ורב־שכבתית, שמאחדת כמה שיטות שחזור במקביל:
- שחזור מבוסס metadata
- שחזור מבוסס filesystem
- partition recovery
- carving לפי חתימות
- reconstruction לקבצים fragmented
- validation לתקינות התוכן
- dedup + fusion לתוצאות
- scoring לאמינות השחזור
- forensic mode לעבודה מדויקת ומתועדת

המערכת צריכה לדעת לעבוד מול:
- דיסקים פיזיים
- מחיצות
- image files
- RAID / LVM / storage layers בהמשך
- מדיות עם סקטורים פגומים
- סריקות ארוכות עם pause/resume
- השוואה בין סריקות
- שמירה מלאה של תוצאות כדי להמשיך מאותה נקודה

---

## 3. עקרונות יסוד

### 3.1 לא עוד "סורק"
המערכת לא תהיה "deep scan" יחיד. זו תהיה פלטפורמה מרובת מנועים.

### 3.2 שמירת תוצאות היא חלק מהליבה
כל סריקה תישמר כמבנה נתונים מלא, עם אפשרות להמשך, השוואה, export/import וניתוח מאוחר.

### 3.3 Rust-first
הליבה תהיה ב־Rust. לא נכניס C++ מוקדם מדי אלא רק כשיש הצדקה אמיתית.

### 3.4 MVP אמיתי
הגרסה הראשונה חייבת להיות קטנה מספיק כדי להיבנות, אבל חזקה מספיק כדי להיות שימושית.

### 3.5 UI מסודר בעברית
הממשק יוגדר מראש כ־RTL מלא, ולא כתוספת מאוחרת.

### 3.6 GitHub הוא ציר העבודה
כל התקדמות, שינוי, roadmap, issue, branch ו־PR יתנהלו דרך GitHub.

---

## 4. בחירת טכנולוגיות סופית

### 4.1 ליבת המנוע
- **Rust** — השפה הראשית
- סיבות:
  - בטיחות זיכרון
  - ביצועים גבוהים
  - שליטה טובה ב־I/O
  - concurrency יציב
  - ecosystem טוב ל־CLI, serialization, SQLite, plugins, async ו־testing

### 4.2 ממשק משתמש
- **Tauri + React + TypeScript**
- סיבות:
  - קל משמעותית מ־Electron
  - מתאים לדסקטופ
  - משתלב טבעית עם ליבת Rust
  - footprint נמוך יותר
  - קל יותר לאריזה ולהפצה

### 4.3 מסד נתונים פנימי
- **SQLite**
- סיבות:
  - בסיס מצוין ל־local state
  - נוח לשמירת scan sessions
  - קל לייצוא/ייבוא
  - מתאים מאוד לאפליקציית דסקטופ
  - מאפשר FTS5 לחיפוש

### 4.4 שפות עזר
- **Python** לכלי בדיקה, ייצור datasets, scripts, benchmark automation, corpus validation
- **SQL** ל־schema, migrations, reports
- **TypeScript** ל־UI types, state management, IPC client layer

### 4.5 C/C++
לא חלק מה־MVP כברירת מחדל. ייכנס רק אם:
- יש parser חיצוני איכותי במיוחד
- נדרש קישור לספריית low-level קיימת
- יש ROI ברור מול מימוש ב־Rust

---

## 5. ארכיטקטורת־על של המערכת

המערכת תחולק ל־12 שכבות עיקריות.

### 5.1 Acquisition Layer
אחריות:
- גישה לקריאה בלבד למדיה
- קריאה של raw devices
- קריאה של image files
- retry policy
- bad sector handling
- checksum לקריאות נדרשות
- pause/resume
- checkpointing

### 5.2 Storage Abstraction Layer
אחריות:
- הפשטת מקור הנתונים
- דיסק פיזי / partition / image / virtual disk
- block addressing אחיד
- metadata בסיסי על המקור

### 5.3 Disk Layout Layer
אחריות:
- זיהוי MBR / GPT
- ניתוח partition entries
- חיפוש lost partitions
- validation למבני partition
- boot sector analysis

### 5.4 Filesystem Parsers Layer
MVP:
- NTFS
- FAT16/32
- exFAT
- ext4

שלבים מתקדמים:
- APFS
- HFS+
- ReFS
- XFS
- Btrfs

### 5.5 Metadata Recovery Layer
אחריות:
- שמות קבצים
- נתיבי תיקיות
- timestamps
- parent/child graph
- orphan recovery
- deleted flags
- confidence לשדות metadata

### 5.6 Carving Layer
אחריות:
- header/footer carving
- structure-aware carving
- size-field carving
- sliding-window detection
- block-aligned carving
- custom signatures

### 5.7 Fragment Reconstruction Layer
אחריות:
- איתור fragments
- adjacency scoring
- reconstruction heuristics
- format-aware assembly
- recovery של קבצים fragmented

MVP formats:
- JPEG
- ZIP / Office containers
- PDF
- SQLite
- MP4 בסיסי

### 5.8 Validation Layer
אחריות:
- בדיקת תקינות תוכן
- classify: intact / partial / corrupt / false positive
- preview-safe extraction
- optional repair hints

### 5.9 Dedup + Fusion Layer
אחריות:
- זיהוי כפילויות
- מיזוג תוצאות מכמה מנועים
- קישור בין metadata result לבין carved result
- בחירת "הנציג הטוב ביותר" לכל artifact

### 5.10 Scoring Layer
אחריות:
- חישוב אמינות
- ציון A–F
- breakdown לפי מדדים
- reason codes

### 5.11 Result Store Layer
אחריות:
- שמירת כל תוצאות הסריקה
- sessions
- findings
- checkpoints
- exports
- imports
- diffs בין סריקות
- חיפוש היסטורי
- audit trail

### 5.12 UI / Workflow Layer
אחריות:
- wizard
- תצוגת מקורות
- תצוגת tree
- תצוגת תוצאות raw
- filters
- preview
- export
- forensic reporting

---

## 6. מבנה monorepo מומלץ

```text
recovery-platform/
  apps/
    desktop/
  crates/
    app-core/
    types/
    core-io/
    acquisition/
    storage-abstraction/
    disk-layout/
    fs-ntfs/
    fs-fat/
    fs-exfat/
    fs-ext4/
    metadata-recovery/
    carver/
    fragment-reconstructor/
    validator/
    dedup-fusion/
    scoring/
    result-store/
    preview-service/
    forensic-report/
    plugin-api/
    job-orchestrator/
  docs/
  scripts/
  testdata/
  migrations/
  .github/
```

---

## 7. אפיון ממשק המשתמש בעברית

### 7.1 עקרונות UI
- RTL מלא
- עברית כברירת מחדל
- dark mode
- תצוגה טכנית אך מסודרת
- נראות ברורה של status, progress, warnings ו־confidence

### 7.2 מסכים עיקריים

#### מסך 1 — פתיחה / פרויקט
יכלול:
- יצירת פרויקט חדש
- פתיחת פרויקט קיים
- טעינת בסיס נתוני סריקה קיים
- ייבוא snapshot
- רשימת סריקות אחרונות

#### מסך 2 — בחירת מקור
יכלול:
- דיסקים מחוברים
- מחיצות
- image files
- מידע בסיסי על כל מקור
- אינדיקציה אם נדרשות הרשאות גבוהות

#### מסך 3 — הגדרות סריקה
יכלול:
- סריקת metadata
- סריקת carving
- סריקת partition
- validators
- forensic mode
- scan profile: מהיר / רגיל / עמוק / מותאם אישית
- שמירת תוצאות ביניים

#### מסך 4 — התקדמות סריקה
יכלול:
- progress כללי
- progress לפי מנוע
- throughput
- errors / bad sectors
- checkpoints
- pause / resume / stop

#### מסך 5 — תוצאות
תתי תצוגות:
- עץ קבצים מקורי
- orphaned / lost paths
- תוצאות לפי סוג קובץ
- carved results
- merged results
- duplicates
- invalid / suspicious

#### מסך 6 — תצוגת פריט
יכלול:
- שם
- נתיב מקורי אם קיים
- מקור הגילוי
- גודל
- timestamps
- hash
- confidence
- מצב תקינות
- preview בטוח
- היסטוריית מיזוג

#### מסך 7 — השוואת סריקות
יכלול:
- scan A מול scan B
- artifacts חדשים
- artifacts שנעלמו
- confidence changes
- parser improvements

#### מסך 8 — דוחות / ייצוא
יכלול:
- export selected files
- export metadata report
- export forensic evidence package
- export scan DB snapshot

---

## 8. שמירת תוצאות סריקה — דרישת ליבה

זו פונקציה חובה מהיום הראשון.

### 8.1 מה חייב להישמר
- פרויקט
- מקור סריקה
- הגדרות סריקה
- גרסת המנוע
- זמני התחלה/סיום
- checkpoints
- events
- raw findings
- merged findings
- validation results
- scoring results
- preview metadata
- export history

### 8.2 יכולות נדרשות
- pause/resume
- reopen project later
- import/export scan database
- snapshot לפני merge
- snapshot אחרי merge
- compare two scans
- recover state after crash

### 8.3 פורמטי שמירה
- SQLite database ראשי
- optional JSON export לסיכומים
- optional zipped project package

---

## 9. סכמת נתונים ראשונית — רמה קונספטואלית

טבלאות עיקריות:
- projects
- scan_sources
- scan_sessions
- scan_profiles
- scan_jobs
- scan_checkpoints
- scan_events
- artifacts
- artifact_names
- artifact_paths
- artifact_fragments
- artifact_sources
- artifact_hashes
- validation_results
- scoring_results
- merge_groups
- exports
- preview_cache
- audit_log

### אינדקסים חשובים
- artifact_hashes(hash_sha256)
- artifacts(size_bytes)
- scan_sessions(project_id, started_at)
- artifact_names(name_normalized)
- artifact_paths(path_normalized)
- FTS5 על שמות/נתיבים/notes

---

## 10. מודולי ליבה ומה כל אחד עושה

### app-core
שכבת orchestration מרכזית.

### types
כל ה־shared models וה־DTOs.

### core-io
קריאה low-level, retries, block streaming.

### acquisition
מדיניות גישה למדיה, checkpoints, throughput monitoring.

### storage-abstraction
הפשטת disk/image/partition.

### disk-layout
GPT/MBR detection וניתוח מחיצות.

### fs-ntfs / fs-fat / fs-exfat / fs-ext4
parsers ייעודיים.

### metadata-recovery
בניית שמות/נתיבים/קשרי תיקיות.

### carver
חיפוש קבצים לפי חתימות ותוכן.

### fragment-reconstructor
איחוי fragments.

### validator
בדיקת תקינות תוכן.

### dedup-fusion
איחוד תוצאות והכרעה בין מועמדים.

### scoring
מתן ציון אמינות מפורט.

### result-store
SQLite access, persistence, migrations, query layer.

### preview-service
יצירת preview בטוח ומוגבל.

### forensic-report
דוחות audit/evidence.

### plugin-api
חוזים עתידיים ל־plugins.

### job-orchestrator
ניהול תורים, משאבים, ביטולים, resume.

---

## 11. מודל scoring

ציון סופי: A–F

מדדים מוצעים:
- metadata confidence
- path confidence
- integrity confidence
- fragmentation risk
- overwrite risk
- parser confidence
- validator confidence
- source reliability

התוצאה תוצג גם כציון וגם כפירוק מפורט.

---

## 12. מודל validators

MVP validators:
- טקסט / UTF-8 / UTF-16
- JPEG
- PNG
- PDF
- ZIP
- DOCX/XLSX/PPTX
- SQLite
- MP4
- JSON/XML בסיסי

כל validator יחזיר:
- status
- notes
- optional repair hints
- preview metadata

---

## 13. מודל dedup + fusion

רמות dedup:
- exact hash
- same ext + same size + overlapping offsets
- fuzzy similarity
- metadata-linkage

מטרת fusion:
- לחבר בין כמה מופעים של אותו artifact
- לבחור best representative
- לשמור provenance מלא

---

## 14. אבטחה והרשאות

### 14.1 גישה לדיסק
צריך להגדיר מהיום הראשון:
- Windows: run as admin / raw device access
- Linux: root / udev / capabilities
- macOS: elevated helper + constraints

### 14.2 תצוגה מקדימה בטוחה
הכלל:
- preview לא יריץ קובץ חיצוני
- ברירת מחדל: טקסט, hex, metadata, thumbnails מחושבים
- PDF/Office preview רק דרך renderer מבודד בעתיד

### 14.3 forensic mode
במצב זה:
- read-only enforced
- audit מלא
- hashing קריטי
- export evidence package

---

## 15. תזמור משאבים וביצועים

צריך מנגנון מסודר ל:
- thread pools נפרדים ל־I/O ול־CPU
- הגבלת זיכרון
- throttling
- backpressure
- pause/resume per job
- priority queues
- long scan stability

---

## 16. Roadmap מומלץ

### Phase 0 — Foundation
מטרה:
- monorepo
- CI
- code style
- schema ראשונית
- result-store ראשוני
- app shell

### Phase 1 — MVP Technical Core
מטרה:
- acquisition
- storage abstraction
- disk-layout
- NTFS/FAT/exFAT/ext4 parsers בסיסיים
- result persistence
- basic carving
- validators בסיסיים
- UI ראשוני

### Phase 2 — Recovery Quality Upgrade
מטרה:
- fragment reconstruction
- dedup/fusion מתקדם
- scoring A–F
- compare scans
- export snapshots

### Phase 3 — Pro / Forensic Expansion
מטרה:
- forensic mode מלא
- audit package
- APFS/HFS+
- advanced reports
- plugins

---

## 17. MVP ריאלי מומלץ

MVP חייב לכלול:
- פתיחת פרויקט
- בחירת מקור
- raw read-only acquisition
- GPT/MBR detection
- parser בסיסי ל־NTFS
- parser בסיסי ל־FAT/exFAT
- ext4 basic listing
- basic metadata recovery
- carving בסיסי ל־JPEG/PNG/PDF/ZIP/TXT
- שמירת תוצאות סריקה מלאה
- pause/resume
- tree view + raw view
- export selected files

לא להכניס ל־MVP:
- AI ranking
- distributed scanning
- plugins dynamic מורכבים
- RAID reconstruction מלא
- APFS מלא אם זה דוחה את ה־MVP

---

## 18. איך מחלקים את העבודה בינינו

### מה אני ממליץ לעשות כאן
אני אבנה בשבילך:
- מסמכי אפיון
- schema
- backlog
- חוזי API
- שלדי קוד
- מודולים ראשוניים
- refactors
- code review
- איחוד החלטות ארכיטקטורה
- תכנון GitHub workflow

### מה אתה תעשה בצד שלך
- תפתח repository ב־GitHub
- תעלה commits באופן מסודר
- תסנכרן שינויים
- תשתף קבצים/דיפים/בעיות בנייה
- תמשוך את ההתקדמות למסגרת אמיתית של repo

### איך עובדים נכון
- מסמך זה הוא source of truth
- כל מודול מקבל תיקיית docs קטנה משלו
- כל feature מקבל issue
- כל שינוי משמעותי נעשה ב־branch
- merge רק עם checklist

---

## 19. GitHub Workflow מומלץ

### Branches
- main
- develop
- feature/*
- fix/*
- research/*

### Pull Requests
כל PR יכלול:
- מה נוסף
- מה השתנה
- מה לא גמור
- migration impact
- test status
- known risks

### Labels
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

### Projects / Milestones
- Foundation
- MVP Core
- Recovery Quality
- Forensic Expansion

---

## 20. סדר העבודה שאני ממליץ עליו עכשיו

### שלב 1
לנעול מסמך זה כמסמך הראשי.

### שלב 2
להקים GitHub repo לפי השלד.

### שלב 3
לבנות 4 נכסים ראשונים:
- schema מלאה
- result-store
- core types
- core-io

### שלב 4
לבנות disk-layout.

### שלב 5
לבנות fs-ntfs ראשון.

### שלב 6
לבנות UI בסיסי רק להצגת sessions ותוצאות.

### שלב 7
להוסיף carver בסיסי.

---

## 21. מה עדיין לא סוגרים סופית עכשיו

נשאיר להמשך:
- plugins ABI קשיח
- C++ integration רחב
- APFS מלא
- distributed scan
- cloud sync
- AI/ML fragment ranking

---

## 22. החלטות סופיות כרגע

1. הליבה ב־Rust  
2. ה־UI ב־Tauri + React + TypeScript  
3. SQLite הוא מרכז המצב והתוצאות  
4. שמירת תוצאות סריקה היא חובה מהיום הראשון  
5. מתחילים מ־MVP ממוקד, לא ממערכת ענק לא גמורה  
6. GitHub הוא ציר העבודה והסנכרון  
7. המסמך הזה הוא מסמך העבודה הראשי  

---

## 23. המשך מומלץ מיידי

הקבצים הבאים צריכים להיכתב מיד אחרי המסמך הזה:
1. `ARCHITECTURE_HE.md` — פירוק טכני עמוק יותר  
2. `DB_SCHEMA_V1.sql` — schema מלאה ל־SQLite  
3. `BACKLOG_MVP_HE.md` — backlog לפי phases  
4. `API_CONTRACTS_HE.md` — חוזים בין מודולים  
5. `GITHUB_WORKFLOW_HE.md` — איך עובדים בפועל  
6. `UI_SPEC_HE.md` — מסכי UI בעברית  

---

