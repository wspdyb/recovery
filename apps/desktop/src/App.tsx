import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { motion, AnimatePresence } from "motion/react";
import {
  Activity,
  Database,
  FileText,
  Folder,
  ChevronRight,
  ChevronDown,
  Download,
  Search,
  Terminal,
  Cpu,
  Shield,
  Eye
} from "lucide-react";
import { FilePreviewer } from "./components/FilePreviewer";
import "./App.css";

/// <reference types="vite/client" />
/// <reference types="react" />
/// <reference types="react-dom" />

// DataRun is not used in the UI directly, but exists in the backend.

interface DataInfo {
  type: "Resident" | "NonResident";
  value: any; // Vec<u8> for Resident, { allocated_size, real_size, data_runs } for NonResident
}

interface ParsedRecord {
  mft_id: number;
  parent_mft_id: number;
  filename: string | null;
  is_directory: boolean;
  is_deleted: boolean;
  data_info: DataInfo | null;
  mime_type?: string;
  real_size?: number;
  disk_offset?: number;
}

interface ScanProgress {
  percentage: number;
  current_file: string;
  files_found: number;
  record: ParsedRecord | null;
}

interface MftNode extends ParsedRecord {
  children: number[];
}

function App() {
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [scanPath, setScanPath] = useState("\\\\.\\C:");
  const [savePath, setSavePath] = useState("");
  const [fileSystem, setFileSystem] = useState<Record<number, MftNode>>({});
  const [expandedNodes, setExpandedNodes] = useState<Set<number>>(new Set([5])); // Root is usually 5
  const [logs, setLogs] = useState<string[]>([]);
  const [selectedFile, setSelectedFile] = useState<MftNode | null>(null);
  const [currentSessionId, setCurrentSessionId] = useState<number | null>(null);

  useEffect(() => {
    const unlisten = listen<ScanProgress>("scan-progress", (event) => {
      const payload = event.payload;
      setProgress(payload);

      if (payload.record) {
        const record = payload.record;
        setFileSystem((prev: Record<number, MftNode>) => {
          const newState = { ...prev };

          // Create or update node
          if (!newState[record.mft_id]) {
            newState[record.mft_id] = { ...record, children: [] };
          } else {
            newState[record.mft_id] = { ...newState[record.mft_id], ...record };
          }

          // Link to parent
          if (record.parent_mft_id !== undefined && record.parent_mft_id !== record.mft_id) {
            if (!newState[record.parent_mft_id]) {
              newState[record.parent_mft_id] = {
                mft_id: record.parent_mft_id,
                parent_mft_id: 0,
                filename: "...",
                is_directory: true,
                is_deleted: false,
                data_info: null,
                children: []
              };
            }
            if (!newState[record.parent_mft_id].children.includes(record.mft_id)) {
              newState[record.parent_mft_id].children.push(record.mft_id);
            }
          }

          return newState;
        });

        if (payload.current_file && payload.current_file !== "הסריקה הושלמה") {
          setLogs((prev: string[]) => [`[FOUND] ${payload.current_file}`, ...prev].slice(0, 100));
        }
      }

      if (payload.percentage >= 100) {
        setIsScanning(false);
        setLogs((prev: string[]) => [`[SYSTEM] Scan completed.`, ...prev]);
        // Trigger final sync if we have a session ID
        // Note: currentSessionId might not be updated yet due to closure,
        // but we can rely on the fact that start_scan is still running or finished.
      }
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  async function handleStartScan() {
    setIsScanning(true);
    setProgress(null);
    setFileSystem({});
    setLogs([`[SYSTEM] Starting scan on ${scanPath}...`]);
    try {
      const sessionId = await invoke("start_scan", {
        path: scanPath,
        savePath: savePath || null
      }) as number;
      setCurrentSessionId(sessionId);
      await syncFromDb(sessionId);
    } catch (e: any) {
      setLogs((prev: string[]) => [`[ERROR] ${e?.message || e}`, ...prev]);
      setIsScanning(false);
    }
  }

  async function syncFromDb(sessionId: number) {
    setLogs((prev: string[]) => [`[SYSTEM] Syncing final results from database (Session ${sessionId})...`, ...prev]);
    try {
      const artifacts = await invoke("get_artifacts", { sessionId }) as any[];
      setFileSystem((prev: Record<number, MftNode>) => {
        const newState = { ...prev };
        artifacts.forEach(art => {
          if (!newState[art.mft_id]) {
            newState[art.mft_id] = {
              ...art,
              is_directory: art.is_dir,
              data_info: null,
              children: []
            };
          } else {
            newState[art.mft_id] = { ...newState[art.mft_id], ...art, is_directory: art.is_dir };
          }

          if (art.parent_mft_id !== undefined && art.parent_mft_id !== art.mft_id) {
            if (!newState[art.parent_mft_id]) {
              newState[art.parent_mft_id] = {
                mft_id: art.parent_mft_id,
                parent_mft_id: 0,
                filename: "...",
                is_directory: true,
                is_deleted: false,
                data_info: null,
                children: []
              };
            }
            if (!newState[art.parent_mft_id].children.includes(art.mft_id)) {
              newState[art.parent_mft_id].children.push(art.mft_id);
            }
          }
        });
        return newState;
      });
      setLogs((prev: string[]) => [`[SUCCESS] Sync complete.`, ...prev]);
    } catch (e: any) {
      setLogs((prev: string[]) => [`[ERROR] Sync failed: ${e}`, ...prev]);
    }
  }

  const toggleNode = (id: number) => {
    setExpandedNodes((prev: Set<number>) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleSelectFile = (node: MftNode) => {
    if (!node.is_directory) {
      setSelectedFile(node);
    }
  };

  const renderTreeNode = (id: number, depth = 0) => {
    const node = fileSystem[id];
    if (!node || (!node.filename && id !== 5)) return null;

    const isExpanded = expandedNodes.has(id);
    const isSelected = selectedFile?.mft_id === id;
    const hasChildren = node.children.length > 0;

    return (
      <div key={id} className="tree-node-wrapper">
        <div
          className={`tree-node ${node.is_directory ? 'is-dir' : 'is-file'} ${isSelected ? 'selected' : ''}`}
          style={{ paddingLeft: `${depth * 16 + 8}px` }}
          onClick={() => {
            if (node.is_directory) {
              toggleNode(id);
            } else {
              handleSelectFile(node);
            }
          }}
        >
          <span className="node-icon">
            {node.is_directory ? (
              isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />
            ) : (
              <FileText size={14} className="text-dim" />
            )}
          </span>
          <span className="node-type-icon">
            {node.is_directory ? <Folder size={14} className="text-accent" /> : <Shield size={14} className="text-primary" />}
          </span>
          <span className="node-name">
            {node.filename || (id === 5 ? "ROOT" : "UNKNOWN")}
            {node.is_deleted && <span className="deleted-badge">מחיקה</span>}
          </span>
          <span className="node-id">ID:{id}</span>
          {!node.is_directory && <Eye size={12} className="preview-indicator" />}
        </div>

        <AnimatePresence>
          {isExpanded && hasChildren && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: "auto", opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              className="node-children"
            >
              {node.children.sort((a: number, b: number) => {
                const nodeA = fileSystem[a];
                const nodeB = fileSystem[b];
                if (nodeA?.is_directory && !nodeB?.is_directory) return -1;
                if (!nodeA?.is_directory && nodeB?.is_directory) return 1;
                return (nodeA?.filename || "").localeCompare(nodeB?.filename || "");
              }).map((childId: number) => renderTreeNode(childId, depth + 1))}
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    );
  };

  return (
    <div className="app-shell" style={{ direction: 'rtl' }}>
      <nav className="side-nav">
        <div className="nav-brand">
          <Activity className="brand-icon" />
          <span>PHOENIX</span>
        </div>
        <div className="nav-items">
          <div className="nav-item active"><Database size={20} /></div>
          <div className="nav-item"><Search size={20} /></div>
          <div className="nav-item"><Download size={20} /></div>
          <div className="nav-item"><Terminal size={20} /></div>
        </div>
        <div className="nav-footer">
          <Cpu size={20} className="text-dim" />
        </div>
      </nav>

      <main className="main-content">
        <header className="main-header">
          <div className="header-left">
            <h1 className="page-title">PHEONIX RECOVER / NTFS ANALYZER</h1>
            <div className="status-badge">
              <div className={`status-dot ${isScanning ? 'pulse' : ''}`} />
              {isScanning ? "מבצע סריקת כונן..." : "מערכת מוכנה"}
            </div>
          </div>
          <div className="header-controls">
            <div className="input-field">
              <label>כונן מקור</label>
              <input
                type="text"
                value={scanPath}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setScanPath(e.target.value)}
                placeholder="\\\\.\\C: or disk.img"
              />
            </div>
            <div className="input-field">
              <label>נתיב שחזור</label>
              <input
                type="text"
                value={savePath}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setSavePath(e.target.value)}
                placeholder="תיקיית פלט..."
              />
            </div>
            <button
              className={`action-btn ${isScanning ? 'disabled' : ''}`}
              onClick={handleStartScan}
              disabled={isScanning}
            >
              {isScanning ? "סורק..." : "התחל סריקה"}
            </button>
            {currentSessionId && !isScanning && (
              <button
                className="action-btn secondary"
                onClick={() => syncFromDb(currentSessionId)}
              >
                רענון מה-DB
              </button>
            )}
          </div>
        </header>

        <div className="dashboard-grid glass">
          <section className="panel tree-panel">
            <div className="panel-header">
              <Folder size={16} />
              <h2>היררכיית קבצים</h2>
            </div>
            <div className="panel-content tree-viewport">
              {Object.keys(fileSystem).length > 0 ? (
                // Find potential roots (nodes without parents or parent not in filesystem)
                Object.keys(fileSystem).filter(id => {
                  const node = fileSystem[parseInt(id)];
                  return node.parent_mft_id === 0 || !fileSystem[node.parent_mft_id];
                }).map(id => renderTreeNode(parseInt(id)))
              ) : (
                <div className="empty-state">
                  <Database size={48} className="text-dim shadow-icon" />
                  <p>אין נתונים. התחל סריקה כדי לראות קבצים.</p>
                </div>
              )}
            </div>
          </section>

          <div className="right-stack">
            {selectedFile ? (
              <section className="panel preview-panel active">
                <div className="panel-header" style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <Eye size={16} />
                    <h2>תצוגה מקדימה</h2>
                  </div>
                  <button
                    onClick={() => setSelectedFile(null)}
                    style={{ background: 'none', border: 'none', color: '#666', cursor: 'pointer' }}
                  >
                    סגור X
                  </button>
                </div>
                <div className="panel-content">
                  <FilePreviewer
                    file={{
                      mft_id: selectedFile.mft_id,
                      filename: selectedFile.filename || "unknown",
                      mime: selectedFile.mime_type
                    }}
                    drive={scanPath}
                  />
                </div>
              </section>
            ) : (
              <section className="panel stats-panel">
                <div className="panel-header">
                  <Activity size={16} />
                  <h2>סטטיסטיקת סריקה</h2>
                </div>
                <div className="panel-content stats-grid">
                  <div className="stat-card">
                    <label>התקדמות</label>
                    <div className="stat-value">{(progress?.percentage || 0).toFixed(1)}%</div>
                    <div className="progress-bar-bg">
                      <motion.div
                        className="progress-bar-fill"
                        initial={{ width: 0 }}
                        animate={{ width: `${progress?.percentage || 0}%` }}
                      />
                    </div>
                  </div>
                  <div className="stat-card">
                    <label>קבצים שזוהו</label>
                    <div className="stat-value">{progress?.files_found || 0}</div>
                  </div>
                </div>
              </section>
            )}

            <section className="panel log-panel">
              <div className="panel-header">
                <Terminal size={16} />
                <h2>יומן מערכת</h2>
              </div>
              <div className="panel-content log-viewport">
                {logs.map((log: string, i: number) => (
                  <div key={i} className="log-entry">
                    <span className="log-time">[{new Date().toLocaleTimeString()}]</span>
                    <span className="log-text">{log}</span>
                  </div>
                ))}
              </div>
            </section>
          </div>
        </div>
      </main>
    </div>
  );
}

export default App;
