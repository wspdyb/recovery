# מפרט ממשק המשתמש

גרסה: 1.0  
תאריך: 12 באפריל 2026  
שפת עבודה: עברית

## עקרונות UI

- **RTL מלא**: כל הטקסט והפריסה מימין לשמאל
- **עברית כברירת מחדל**: אין תרגום, הכל בעברית
- **Dark Mode**: ערכת צבעים כהה כברירת מחדל
- **טכני אך מסודר**: הצגת נתונים טכניים בצורה ברורה
- **אינדיקציה ברורה**: status, progress, warnings, confidence

## טכנולוגיות

- **Framework**: React + TypeScript
- **UI Library**: TBD (שקול: Mantine, Chakra UI, Material-UI)
- **State Management**: Zustand או Redux Toolkit
- **Styling**: CSS-in-JS או Tailwind CSS
- **Icons**: Lucide React או Heroicons
- **Charts**: Recharts ל-progress

## מבנה אפליקציה

```
src/
  components/
    common/
    screens/
  hooks/
  stores/
  types/
  utils/
  ipc/
```

## מסכים עיקריים

### 1. מסך פתיחה / פרויקטים

**מטרה**: ניהול פרויקטים וסשנים

**רכיבים**:
- Header עם לוגו וכותרת
- כפתור "פרויקט חדש"
- רשימת פרויקטים אחרונים
- כפתור "פתח פרויקט קיים"
- כפתור "ייבא snapshot"

**פונקציונליות**:
- יצירת פרויקט חדש עם שם ותיאור
- פתיחת פרויקט קיים מרשימה
- ייבוא DB snapshot
- הצגת metadata: תאריך יצירה, מספר סריקות, גודל

**ניווט**: לוחצים על פרויקט → מסך בחירת מקור

### 2. מסך בחירת מקור

**מטרה**: בחירת מקור לסריקה

**רכיבים**:
- רשימת דיסקים מחוברים
- רשימת image files (DD, E01)
- פרטים על כל מקור: שם, גודל, סוג
- אינדיקציה אם דורש הרשאות admin
- כפתור "רענן רשימה"

**פונקציונליות**:
- זיהוי אוטומטי של דיסקים
- בחירת קובץ image דרך file picker
- הצגת partitions אם קיים
- אזהרה על read-only

**ניווט**: בוחרים מקור → מסך הגדרות סריקה

### 3. מסך הגדרות סריקה

**מטרה**: הגדרת פרופיל סריקה

**רכיבים**:
- Checkboxes למנועים:
  - [ ] סריקת metadata
  - [ ] סריקת carving
  - [ ] סריקת partition
  - [ ] validators
  - [ ] forensic mode
- בחירת scan profile: מהיר / רגיל / עמוק / מותאם אישית
- Toggle "שמירת תוצאות ביניים"
- כפתור "התחל סריקה"

**פונקציונליות**:
- שמירת profiles מותאמים
- preview של משך זמן משוער
- אזהרות על forensic mode

**ניווט**: "התחל סריקה" → מסך התקדמות

### 4. מסך התקדמות סריקה

**מטרה**: הצגת התקדמות בזמן אמת

**רכיבים**:
- Progress bar כללי (0-100%)
- Progress לפי מנוע (metadata, carving, etc.)
- Throughput: MB/s, files/s
- סטטיסטיקות: bad sectors, errors
- רשימת events אחרונים
- כפתורים: Pause / Resume / Stop
- Estimated time remaining

**פונקציונליות**:
- עדכון בזמן אמת דרך IPC events
- הצגת warnings/errors
- Checkpointing אוטומטי
- Cancel עם confirmation

**ניווט**: סיום סריקה → מסך תוצאות

### 5. מסך תוצאות

**מטרה**: הצגת תוצאות הסריקה

**תתי תצוגות**:

#### Tree View
- עץ תיקיות מקורי
- orphaned paths
- הצגת confidence בצבעים
- Context menu: preview, export

#### Raw View
- טבלה עם כל artifacts
- עמודות: שם, גודל, offset, confidence, score
- סינון לפי סוג קובץ
- Sortable columns

#### Carved Results
- רשימה לפי signature
- תצוגה מקדימה

#### Merged Results
- קבוצות dedup
- best representative

#### Duplicates
- רשימת כפילויות
- merge options

#### Invalid/Suspicious
- artifacts עם low confidence
- reasons

**פונקציונליות**:
- חיפוש ב-FTS
- פילטרים מתקדמים
- bulk selection
- export selected

**ניווט**: לחיצה על artifact → מסך פריט

### 6. מסך תצוגת פריט

**מטרה**: פרטים על artifact בודד

**רכיבים**:
- שם ו-path מקורי
- מקור הגילוי (metadata/carving)
- גודל, offsets, timestamps
- Hash (SHA256)
- מצב תקינות (valid/partial/corrupt)
- Confidence score עם breakdown
- Preview: text/hex/thumbnail
- היסטוריית מיזוג

**פונקציונליות**:
- Preview בטוח (לא מריץ קבצים)
- Export single file
- View raw data
- Compare versions אם merged

**ניווט**: חזרה לרשימה

### 7. מסך השוואת סריקות

**מטרה**: השוואה בין סריקות

**רכיבים**:
- בחירת scan A ו-scan B
- Tabs:
  - New artifacts
  - Missing artifacts
  - Changed confidence
  - Parser improvements
- Diff view עם highlights

**פונקציונליות**:
- Side-by-side comparison
- Export diff report
- Merge improvements

**ניווט**: Standalone או מרשימת סריקות

### 8. מסך דוחות / ייצוא

**מטרה**: ייצוא תוצאות

**רכיבים**:
- Export selected files (zip)
- Export metadata report (CSV/JSON)
- Export forensic evidence package
- Export scan DB snapshot
- Options: compression, password protection

**פונקציונליות**:
- Progress bar לייצוא גדול
- Verification של exported files
- Audit trail

**ניווט**: Standalone

## Components משותפים

### Header
- Navigation breadcrumbs
- Search bar
- User actions (settings, about)

### Sidebar
- Project info
- Quick stats
- Recent scans

### Status Indicators
- Confidence: צבעים ירוק/צהוב/אדום
- Progress: animated bars
- Errors: icons עם tooltips

### Modals/Dialogs
- Confirmation dialogs
- Progress modals
- Error messages
- Settings

## Responsive Design

- Desktop: full layout
- Tablet: collapsible sidebar
- Mobile: stack layout, simplified

## Accessibility

- Keyboard navigation
- Screen reader support
- High contrast mode
- Focus indicators

## Performance

- Virtualized lists ל-big datasets
- Lazy loading של previews
- Debounced search
- Background IPC calls

## Security UI

- No execution of files
- Sandboxed previews
- Clear security warnings
- Audit log viewer

## Future Extensions

- Plugin UI
- Advanced filtering
- Custom dashboards
- Batch operations