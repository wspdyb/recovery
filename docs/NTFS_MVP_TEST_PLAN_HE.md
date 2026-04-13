# תוכנית בדיקות: PhoenixRecover NTFS MVP

מסמך זה מגדיר את מקרי הבדיקה, תרחישי האימות והקריטריונים להצלחה עבור גרסת ה-MVP של PhoenixRecover.

## 1. תשתית הבדיקות (Test Corpus)
כל מקרי הבדיקה מבוססים על קבצי Image הממוקמים ב-`testdata/ntfs/`.

### 1.1 תרחיש: סריקה תקינה (tiny_valid)
- **מטרה**: לוודא שהמערכת מזהה NTFS, קוראת MFT ומוצאת קבצים קיימים.
- **מקור**: `testdata/ntfs/tiny_valid/valid.img`
- **פקודה**: `cargo run -p app-core -- --source testdata/ntfs/tiny_valid/valid.img`
- **קריטריון הצלחה**:
    - זיהוי NTFS מוצלח.
    - מציאת לפחות 2 ארטיפקטים: `MFT_Valid.bin`, `Document.docx`.
    - סטטוס יציאה: 0.

### 1.2 תרחיש: קבצים מחוקים (tiny_deleted)
- **מטרה**: לוודא זיהוי נכון של דגלי מחיקה ב-MFT.
- **מקור**: `testdata/ntfs/tiny_deleted/deleted.img`
- **קריטריון הצלחה**:
    - מציאת 3 ארטיפקטים.
    - זיהוי 2 מתוכם כ-`is_deleted: true`.
    - שמות: `Deleted1.bak`, `OldData.old`.

### 1.3 תרחיש: דיסק שלם (whole_disk)
- **מטרה**: לוודא שה-partition layout detector מוצא את ה-offset הנכון של ה-NTFS.
- **מקור**: `testdata/ntfs/whole_disk/disk.img`
- **קריטריון הצלחה**:
    - זיהוי MBR Layout.
    - מציאת NTFS ב-offset של 1MB (LBA 2048).
    - סריקה מושלמת של ה-MFT הפנימי.

### 1.4 תרחיש: זיהוי שגוי / קובץ לא תואם (non_ntfs)
- **מטרה**: לוודא מניעת False Positives וטיפול נקי בשגיאות.
- **מקור**: `testdata/ntfs/non_ntfs/random.img`
- **קריטריון הצלחה**:
    - הודעת שגיאה: "NTFS not found" או "FileSystemTypeMismatch".
    - סטטוס יציאה: 2 (NTFS Detection Failure).

## 2. אימות פלט ו-CLI
- [ ] וידוא שכל הרצה מייצרת סיכום JSON אחיד בסוף.
- [ ] וידוא שקודי היציאה (Exit Codes) תואמים להגדרות:
    - 0: הצלחה מלאה.
    - 1: שגיאת קלט (קובץ לא קיים).
    - 2: NTFS לא זוהה.
    - 3: שגיאת פיענוח (Parse Error).
    - 4: שגיאת בסיס נתונים (SQLite/DB).

## 3. אימות בסיס נתונים (SQLite)
- [ ] בדיקת טבלת `artifacts` - האם כל ה-Metadata נשמר (Name, Offset, Size, Deleted, RecordNum).
- [ ] בדיקת טבלת `sessions` - האם פרטי הסריקה נרשמו כראוי.

## 4. אימות ממשק משתמש (UI)
- [ ] פתיחת האפליקציה ב-Desktop.
- [ ] טעינת תוצאות מ-DB אמיתי שנוצר ב-CLI.
- [ ] וידוא שהפילטר "Deleted Only" מציג את הקבצים הנכונים מ-`tiny_deleted`.
