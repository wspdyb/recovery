# Backlog MVP — רשימת משימות לפי שלבים

גרסה: 1.0  
תאריך: 12 באפריל 2026  
שפת עבודה: עברית

## מבוא

מסמך זה מגדיר את ה-backlog ל-MVP, מחולק לפי phases כפי שמתואר במסמך העבודה הראשי. כל משימה כוללת תיאור, קריטריונים לקבלה, ותלויות.

## Phase 0 — Foundation

### משימות

1. **הקמת Monorepo**
   - תיאור: יצירת מבנה תיקיות לפי התוכנית
   - קריטריונים: כל התיקיות קיימות, README.md בסיסי
   - תלויות: אין

2. **הגדרת CI/CD**
   - תיאור: GitHub Actions ל-build, test, lint
   - קריטריונים: Pipeline עובד על push ל-main
   - תלויות: Monorepo

3. **Code Style ו-Linting**
   - תיאור: rustfmt, clippy, eslint
   - קריטריונים: כל הקוד עובר linting
   - תלויות: Monorepo

4. **Schema ראשוני**
   - תיאור: יצירת DB_SCHEMA_V1.sql
   - קריטריונים: כל הטבלאות מוגדרות
   - תלויות: אין

5. **Result-Store ראשוני**
   - תיאור: מודול result-store עם basic CRUD
   - קריטריונים: חיבור ל-SQLite, insert/select
   - תלויות: Schema

6. **Core Types**
   - תיאור: מודול types עם כל ה-structs וה-enums
   - קריטריונים: כל הטייפים מוגדרים
   - תלויות: אין

7. **Core-IO**
   - תיאור: מודול core-io עם block reading
   - קריטריונים: קריאת קבצים בבלוקים
   - תלויות: Types

8. **App Shell**
   - תיאור: Tauri app בסיסי עם חלון ריק
   - קריטריונים: אפליקציה נפתחת
   - תלויות: אין

## Phase 1 — MVP Technical Core

### משימות

9. **Acquisition Layer**
   - תיאור: קריאה מ-raw devices ו-images
   - קריטריונים: retry, bad sectors, pause/resume
   - תלויות: Core-IO

10. **Storage Abstraction**
    - תיאור: BlockReader trait ויישומים
    - קריטריונים: אחידות בין סוגי מקורות
    - תלויות: Acquisition

11. **Disk Layout**
    - תיאור: זיהוי MBR/GPT
    - קריטריונים: partition parsing
    - תלויות: Storage Abstraction

12. **NTFS Parser בסיסי**
    - תיאור: קריאת MFT ותיקיות
    - קריטריונים: רשימת קבצים בסיסית
    - תלויות: Disk Layout

13. **FAT Parser בסיסי**
    - תיאור: קריאת FAT table
    - קריטריונים: רשימת קבצים
    - תלויות: Disk Layout

14. **exFAT Parser בסיסי**
    - תיאור: קריאת allocation bitmap
    - קריטריונים: רשימת קבצים
    - תלויות: Disk Layout

15. **ext4 Parser בסיסי**
    - תיאור: קריאת inode table
    - קריטריונים: רשימת קבצים
    - תלויות: Disk Layout

16. **Metadata Recovery**
    - תיאור: בניית עץ תיקיות
    - קריטריונים: parent/child links
    - תלויות: Parsers

17. **Basic Carving**
    - תיאור: header/footer carving ל-JPEG/PNG/PDF/ZIP/TXT
    - קריטריונים: זיהוי קבצים
    - תלויות: Storage Abstraction

18. **Validators בסיסיים**
    - תיאור: בדיקת תקינות ל-formats נפוצים
    - קריטריונים: status valid/invalid
    - תלויות: Carving

19. **Result Persistence**
    - תיאור: שמירת תוצאות ל-DB
    - קריטריונים: artifacts, names, paths
    - תלויות: Result-Store

20. **Pause/Resume**
    - תיאור: checkpoints ו-state recovery
    - קריטריונים: המשך מסריקה קודמת
    - תלויות: Result Persistence

## Phase 2 — Recovery Quality Upgrade

### משימות

21. **Fragment Reconstruction**
    - תיאור: איחוי fragments
    - קריטריונים: JPEG/PDF reconstruction
    - תלויות: Carving

22. **Dedup/Fusion מתקדם**
    - תיאור: מיזוג תוצאות
    - קריטריונים: merge groups
    - תלויות: Validators

23. **Scoring A-F**
    - תיאור: חישוב ציונים
    - קריטריונים: overall score
    - תלויות: Fusion

24. **Compare Scans**
    - תיאור: השוואת סריקות
    - קריטריונים: diff artifacts
    - תלויות: Scoring

25. **Export Snapshots**
    - תיאור: ייצוא DB snapshots
    - קריטריונים: import/export
    - תלויות: Result Persistence

## Phase 3 — Pro / Forensic Expansion

### משימות

26. **Forensic Mode**
    - תיאור: read-only, audit
    - קריטריונים: evidence package
    - תלויות: Scoring

27. **APFS Parser**
    - תיאור: APFS filesystem support
    - קריטריונים: רשימת קבצים
    - תלויות: Disk Layout

28. **Advanced Reports**
    - תיאור: דוחות audit/evidence
    - קריטריונים: PDF reports
    - תלויות: Forensic Mode

29. **Plugin API**
    - תיאור: חוזים ל-plugins
    - קריטריונים: load external parsers
    - תלויות: App Core

30. **Job Orchestrator**
    - תיאור: ניהול תורים
    - קריטריונים: parallel jobs
    - תלויות: App Core

## UI Tasks (מקביל)

31. **UI Shell**
    - תיאור: Tauri + React בסיסי
    - קריטריונים: חלון עם menu
    - תלויות: App Shell

32. **מסך פרויקטים**
    - תיאור: יצירה/פתיחה של פרויקטים
    - קריטריונים: list projects
    - תלויות: UI Shell

33. **מסך מקורות**
    - תיאור: בחירת disk/image
    - קריטריונים: list sources
    - תלויות: Acquisition

34. **מסך הגדרות סריקה**
    - תיאור: בחירת מנועים
    - קריטריונים: scan profiles
    - תלויות: UI Shell

35. **מסך התקדמות**
    - תיאור: progress bars
    - קריטריונים: real-time updates
    - תלויות: Job Orchestrator

36. **מסך תוצאות**
    - תיאור: tree view + raw view
    - קריטריונים: display artifacts
    - תלויות: Result Persistence

37. **מסך השוואה**
    - תיאור: diff scans
    - קריטריונים: side-by-side
    - תלויות: Compare Scans

38. **מסך ייצוא**
    - תיאור: export options
    - קריטריונים: download files
    - תלויות: Export

## קריטריונים ל-MVP

- פתיחת פרויקט
- בחירת מקור
- raw read-only acquisition
- GPT/MBR detection
- basic NTFS/FAT/exFAT/ext4 listing
- basic metadata recovery
- basic carving ל-JPEG/PNG/PDF/ZIP/TXT
- שמירת תוצאות מלאה
- pause/resume
- tree view + raw view
- export selected files

## סדר עבודה מומלץ

1. Foundation (משימות 1-8)
2. Core Acquisition + Parsers (9-16)
3. Recovery + Persistence (17-20)
4. UI בסיסי (31-36)
5. Quality Upgrade (21-25)
6. Forensic (26-30, 37-38)