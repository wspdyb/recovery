import subprocess
import json
import os
import sys

def run_test(test_dir):
    full_path = os.path.join("testdata", "ntfs", test_dir)
    img_file = next((f for f in os.listdir(full_path) if f.endswith(".img")), None)
    expected_file = os.path.join(full_path, "expected.json")

    if not img_file or not os.path.exists(expected_file):
        print(f"[SKIP] {test_dir} - Missing files")
        return False

    with open(expected_file, "r") as f:
        expected = json.load(f)

    db_path = f"scan_{test_dir}.db"
    if os.path.exists(db_path):
        os.remove(db_path)

    cmd = [
        "cargo", "run", "-p", "app-core", "--",
        "--source", os.path.join(full_path, img_file),
        "--db", db_path,
        "--max-records", "100"
    ]

    print(f"[RUN] Testing {test_dir}...")
    result = subprocess.run(cmd, capture_output=True, text=True)

    # 1. Check Exit Code
    actual_exit = result.returncode
    expect_ntfs = expected.get("expect_ntfs", True)

    success = True

    if expect_ntfs and actual_exit != 0:
        print(f"  [FAIL] Expected exit code 0, got {actual_exit}")
        success = False
    elif not expect_ntfs and actual_exit == 0:
        print(f"  [FAIL] Expected failure for non-NTFS, but got exit code 0")
        success = False

    # 2. Check JSON Summary if successful
    if actual_exit == 0:
        try:
            # Find the JSON block in stdout
            lines = result.stdout.strip().split("\n")
            json_str = ""
            start_json = False
            for line in lines:
                if line.strip() == "{":
                    start_json = True
                if start_json:
                    json_str += line

            summary = json.loads(json_str)

            # Verify artifacts count
            min_artifacts = expected.get("min_artifacts", 0)
            if summary["stored_artifacts"] < min_artifacts:
                print(f"  [FAIL] Expected at least {min_artifacts} artifacts, found {summary['stored_artifacts']}")
                success = False

            # Verify deleted count
            if "has_deleted" in expected:
                if expected["has_deleted"] and summary["deleted_artifacts"] == 0:
                    print(f"  [FAIL] Expected deleted artifacts, found 0")
                    success = False

        except Exception as e:
            print(f"  [FAIL] Failed to parse summary JSON: {e}")
            print(f"  [STDOUT]: {result.stdout}")
            success = False

    if success:
        print(f"  [PASS] {test_dir} verified.")
    return success

def main():
    test_cases = ["tiny_valid", "tiny_deleted", "whole_disk", "corrupt_vbr", "non_ntfs"]
    all_passed = True
    for case in test_cases:
        if not run_test(case):
            all_passed = False

    if all_passed:
        print("\n[COMPLETE] All Integration Tests Passed! PhoenixRecover NTFS MVP Core is Validated.")
        sys.exit(0)
    else:
        print("\n[FAILED] One or more tests failed.")
        sys.exit(1)

if __name__ == "__main__":
    main()
