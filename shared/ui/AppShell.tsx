import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
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
    Eye,
    X,
    Minus,
    Square,
    Copy
} from "lucide-react";
import { FilePreviewer } from "../../apps/desktop/src/components/FilePreviewer";
import "./AppShell.css";

interface DataInfo {
    type: "Resident" | "NonResident";
    value: any;
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

const isTauri =
    typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export default function AppShell() {
    const [progress, setProgress] = useState<ScanProgress | null>(null);
    const [isScanning, setIsScanning] = useState(false);
    const [scanPath, setScanPath] = useState("\\\\.\\C:");
    const [savePath, setSavePath] = useState("");
    const [fileSystem, setFileSystem] = useState<Record<number, MftNode>>({});
    const [expandedNodes, setExpandedNodes] = useState<Set<number>>(new Set([5]));
    const [logs, setLogs] = useState<string[]>([]);
    const [selectedFile, setSelectedFile] = useState<MftNode | null>(null);
    const [currentSessionId, setCurrentSessionId] = useState<number | null>(null);
    const [isMaximized, setIsMaximized] = useState(false);

    useEffect(() => {
        if (!isTauri) return;

        let unlistenResizePromise: Promise<() => void> | null = null;

        async function setupWindowState() {
            try {
                const win = getCurrentWindow();
                setIsMaximized(await win.isMaximized());

                unlistenResizePromise = win.onResized(async () => {
                    try {
                        setIsMaximized(await win.isMaximized());
                    } catch { }
                });
            } catch { }
        }

        setupWindowState();

        return () => {
            if (unlistenResizePromise) {
                unlistenResizePromise.then((f) => f());
            }
        };
    }, []);

    useEffect(() => {
        if (!isTauri) {
            setLogs(["[BROWSER] Running in browser preview mode"]);
            return;
        }

        const unlisten = listen<ScanProgress>("scan-progress", (event) => {
            const payload = event.payload;
            setProgress(payload);

            if (payload.record) {
                const record = payload.record;
                setFileSystem((prev) => {
                    const newState = { ...prev };

                    if (!newState[record.mft_id]) {
                        newState[record.mft_id] = { ...record, children: [] };
                    } else {
                        newState[record.mft_id] = { ...newState[record.mft_id], ...record };
                    }

                    if (
                        record.parent_mft_id !== undefined &&
                        record.parent_mft_id !== record.mft_id
                    ) {
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
                    setLogs((prev) => [`[FOUND] ${payload.current_file}`, ...prev].slice(0, 100));
                }
            }

            if (payload.percentage >= 100) {
                setIsScanning(false);
                setLogs((prev) => ["[SYSTEM] Scan finished.", ...prev]);
            }
        });

        return () => {
            unlisten.then((f) => f());
        };
    }, []);

    async function handleMinimize() {
        if (!isTauri) return;
        try {
            await getCurrentWindow().minimize();
        } catch { }
    }

    async function handleToggleMaximize() {
        if (!isTauri) return;
        try {
            const win = getCurrentWindow();
            const maximized = await win.isMaximized();
            if (maximized) {
                await win.unmaximize();
                setIsMaximized(false);
            } else {
                await win.maximize();
                setIsMaximized(true);
            }
        } catch { }
    }

    async function handleClose() {
        if (!isTauri) return;
        try {
            await getCurrentWindow().close();
        } catch { }
    }

    async function handleStartScan() {
        if (!isTauri) {
            setLogs((prev) => ["[BROWSER] Scan is disabled outside Tauri", ...prev]);
            return;
        }

        setIsScanning(true);
        setProgress(null);
        setFileSystem({});
        setLogs([`[SYSTEM] Starting scan on ${scanPath}...`]);

        try {
            const sessionId = (await invoke("start_scan", {
                path: scanPath,
                savePath: savePath || null
            })) as number;

            setCurrentSessionId(sessionId);
            await syncFromDb(sessionId);
        } catch (e: any) {
            setLogs((prev) => [`[ERROR] ${e?.message || e}`, ...prev]);
            setIsScanning(false);
        }
    }

    async function syncFromDb(sessionId: number) {
        if (!isTauri) return;

        setLogs((prev) => [
            `[SYSTEM] Syncing final results from database (Session ${sessionId})...`,
            ...prev
        ]);

        try {
            const artifacts = (await invoke("get_artifacts", { sessionId })) as any[];

            setFileSystem((prev) => {
                const newState = { ...prev };

                artifacts.forEach((art) => {
                    if (!newState[art.mft_id]) {
                        newState[art.mft_id] = {
                            ...art,
                            is_directory: art.is_dir,
                            data_info: null,
                            children: []
                        };
                    } else {
                        newState[art.mft_id] = {
                            ...newState[art.mft_id],
                            ...art,
                            is_directory: art.is_dir
                        };
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

            setLogs((prev) => ["[SUCCESS] Sync complete.", ...prev]);
        } catch (e: any) {
            setLogs((prev) => [`[ERROR] Sync failed: ${e}`, ...prev]);
        }
    }

    const toggleNode = (id: number) => {
        setExpandedNodes((prev) => {
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

    const renderTreeNode = (id: number, depth = 0): React.ReactNode => {
        const node = fileSystem[id];
        if (!node || (!node.filename && id !== 5)) return null;

        const isExpanded = expandedNodes.has(id);
        const isSelected = selectedFile?.mft_id === id;
        const hasChildren = node.children.length > 0;

        return (
            <div key={id} className="tree-node-wrapper">
                <div
                    className={`tree-node ${node.is_directory ? "is-dir" : "is-file"} ${isSelected ? "selected" : ""}`}
                    style={{ paddingLeft: `${depth * 16 + 8}px` }}
                    onClick={() => {
                        if (node.is_directory) toggleNode(id);
                        else handleSelectFile(node);
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
                        {node.is_directory ? (
                            <Folder size={14} className="text-accent" />
                        ) : (
                            <Shield size={14} className="text-primary" />
                        )}
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
                            {node.children
                                .sort((a, b) => {
                                    const nodeA = fileSystem[a];
                                    const nodeB = fileSystem[b];
                                    if (nodeA?.is_directory && !nodeB?.is_directory) return -1;
                                    if (!nodeA?.is_directory && nodeB?.is_directory) return 1;
                                    return (nodeA?.filename || "").localeCompare(nodeB?.filename || "");
                                })
                                .map((childId) => renderTreeNode(childId, depth + 1))}
                        </motion.div>
                    )}
                </AnimatePresence>
            </div>
        );
    };

    return (
        <div className="window-root">
            <div
                className="custom-titlebar"
                data-tauri-drag-region={isTauri ? true : undefined}
                onDoubleClick={isTauri ? handleToggleMaximize : undefined}
            >
                <div className="titlebar-left" data-tauri-drag-region={isTauri ? true : undefined}>
                    <div className="titlebar-brand" data-tauri-drag-region={isTauri ? true : undefined}>
                        <span className="titlebar-logo">🦅</span>
                        <span className="titlebar-title">PhoenixRecover PRO</span>
                    </div>
                </div>

                {isTauri && (
                    <div className="titlebar-right">
                        <button className="titlebar-btn" type="button" onClick={handleMinimize}>
                            <Minus size={16} />
                        </button>
                        <button className="titlebar-btn" type="button" onClick={handleToggleMaximize}>
                            {isMaximized ? <Copy size={14} /> : <Square size={14} />}
                        </button>
                        <button className="titlebar-btn titlebar-btn-close" type="button" onClick={handleClose}>
                            <X size={16} />
                        </button>
                    </div>
                )}
            </div>

            <div className="app-shell">
                <aside className="side-nav">
                    <div className="nav-brand">
                        <Database className="brand-icon" />
                        <span>PXR</span>
                    </div>
                    <div className="nav-items">
                        <div className="nav-item active"><Activity size={18} /></div>
                        <div className="nav-item"><Search size={18} /></div>
                        <div className="nav-item"><Terminal size={18} /></div>
                        <div className="nav-item"><Cpu size={18} /></div>
                    </div>
                </aside>

                <main className="main-content">
                    <header className="main-header">
                        <div>
                            <h1 className="page-title">PhoenixRecover NTFS MVP</h1>
                            <div className="status-badge">
                                <span className={`status-dot ${isScanning ? "pulse" : ""}`}></span>
                                <span>{isScanning ? "Scanning..." : "Ready"}</span>
                                {currentSessionId && <span>Session #{currentSessionId}</span>}
                            </div>
                        </div>

                        <div className="header-controls">
                            <div className="input-field">
                                <label>SCAN PATH</label>
                                <input value={scanPath} onChange={(e) => setScanPath(e.target.value)} />
                            </div>
                            <div className="input-field">
                                <label>SAVE PATH (Optional)</label>
                                <input value={savePath} onChange={(e) => setSavePath(e.target.value)} />
                            </div>
                            <button
                                className={`action-btn ${isScanning ? "disabled" : ""}`}
                                onClick={handleStartScan}
                                disabled={isScanning}
                            >
                                {isScanning ? "SCANNING..." : "START SCAN"}
                            </button>
                        </div>
                    </header>

                    <div className="dashboard-grid">
                        <section className="panel">
                            <div className="panel-header">
                                <Folder size={14} />
                                <h2>Recovered File Tree</h2>
                            </div>
                            <div className="panel-content tree-viewport">
                                {renderTreeNode(5)}
                            </div>
                        </section>

                        <section className="panel">
                            <div className="panel-header">
                                <FileText size={14} />
                                <h2>Preview / Logs</h2>
                            </div>
                            <div className="panel-content">
                                {selectedFile ? (
                                    <FilePreviewer
                                        file={{
                                            mft_id: selectedFile.mft_id,
                                            filename: selectedFile.filename || "unknown",
                                            mime: selectedFile.mime_type
                                        }}
                                        drive={scanPath}
                                    />
                                ) : (
                                    <div className="log-console">
                                        {logs.map((log, idx) => (
                                            <div key={idx} className="log-line">{log}</div>
                                        ))}
                                        {progress && (
                                            <div className="progress-box">
                                                <div>Progress: {progress.percentage.toFixed(1)}%</div>
                                                <div>Files Found: {progress.files_found}</div>
                                            </div>
                                        )}
                                    </div>
                                )}
                            </div>
                        </section>
                    </div>
                </main>
            </div>
        </div>
    );
}
