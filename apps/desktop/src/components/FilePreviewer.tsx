import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FileImage, FileText, FileVideo, FileAudio, FileCode, Shield, FileArchive } from "lucide-react";

interface FilePreviewerProps {
    file: {
        mft_id: number;
        filename: string;
        mime?: string;
    };
    drive: string;
}

export const FilePreviewer = ({ file, drive }: FilePreviewerProps) => {
    const [hexData, setHexData] = useState<Uint8Array | null>(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const streamUrl = useMemo(() =>
        `stream://localhost/${file.mft_id}?drive=${encodeURIComponent(drive)}&filename=${encodeURIComponent(file.filename)}`,
        [file.mft_id, drive, file.filename]
    );

    useEffect(() => {
        setLoading(true);
        setError(null);
        invoke<number[]>("get_file_preview_data", { mftId: file.mft_id, drive })
            .then((data) => {
                setHexData(new Uint8Array(data));
                setLoading(false);
            })
            .catch((err) => {
                console.error("Failed to load preview data", err);
                setError("נכשל בטעינת נתונים בינאריים");
                setLoading(false);
            });
    }, [file.mft_id, drive]);

    const mime = file.mime || "application/octet-stream";

    const getIconForMime = () => {
        if (mime.startsWith('image/')) return <FileImage size={18} />;
        if (mime.startsWith('video/')) return <FileVideo size={18} />;
        if (mime.startsWith('audio/')) return <FileAudio size={18} />;
        if (mime.startsWith('text/')) return <FileText size={18} />;
        if (mime.includes('zip') || mime.includes('rar') || mime.includes('7z')) return <FileArchive size={18} />;
        if (mime.includes('javascript') || mime.includes('json') || mime.includes('rust')) return <FileCode size={18} />;
        return <Shield size={18} />;
    };

    const renderHexLine = (startIndex: number) => {
        if (!hexData) return null;
        const chunk = hexData.slice(startIndex, startIndex + 16);
        // Use Uint8Array methods or spread to ensure numbers
        const hexPart = (Array.from(chunk as Uint8Array) as number[]).map((b: number) => b.toString(16).padStart(2, '0').toUpperCase()).join(' ');
        const asciiPart = (Array.from(chunk as Uint8Array) as number[]).map((b: number) => (b >= 32 && b <= 126) ? String.fromCharCode(b) : '.').join('');

        return (
            <div key={startIndex} style={{ display: 'flex', gap: '20px', fontFamily: 'monospace', marginBottom: '4px' }}>
                <span style={{ color: '#555' }}>{(startIndex as number).toString(16).padStart(6, '0').toUpperCase()}</span>
                <span style={{ color: '#fff' }}>{hexPart.padEnd(47, ' ')}</span>
                <span style={{ color: '#0f0' }}>{asciiPart}</span>
            </div>
        );
    };

    return (
        <div className="file-previewer" style={{ direction: 'rtl' }}>
            <div className="preview-header" style={{ marginBottom: '15px', borderBottom: '1px solid #333', paddingBottom: '10px', display: 'flex', alignItems: 'center', gap: '10px' }}>
                <div style={{ color: '#0078d4' }}>{getIconForMime()}</div>
                <div>
                    <h3 style={{ margin: 0, color: '#fff', fontSize: '1.1rem' }}>{file.filename}</h3>
                    <code style={{ fontSize: '0.8rem', color: '#888' }}>MIME: {mime} | ID: {file.mft_id}</code>
                </div>
            </div>

            <div className="preview-content" style={{ minHeight: '300px', backgroundColor: '#111', borderRadius: '8px', overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
                {/* 1. Images */}
                {mime.startsWith('image/') && (
                    <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', padding: '20px' }}>
                        <img
                            src={streamUrl}
                            alt={file.filename}
                            style={{ maxWidth: '100%', maxHeight: '400px', objectFit: 'contain', boxShadow: '0 4px 20px rgba(0,0,0,0.5)' }}
                        />
                    </div>
                )}

                {/* 2. Media (Video/Audio) */}
                {(mime.startsWith('video/') || mime.startsWith('audio/')) && (
                    <div className="media-player" style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                        <video
                            controls
                            autoPlay={false}
                            src={streamUrl}
                            style={{ width: '100%', maxHeight: '400px', outline: 'none' }}
                        >
                            הדפדפן שלך לא תומך בנגינת קובץ זה.
                        </video>
                    </div>
                )}

                {/* 3. PDF */}
                {mime === 'application/pdf' && (
                    <iframe
                        src={streamUrl}
                        style={{ width: '100%', height: '500px', border: 'none' }}
                        title="PDF Preview"
                    />
                )}

                {/* 4. Text */}
                {mime.startsWith('text/') && hexData && (
                    <div className="text-preview" style={{ padding: '20px', maxHeight: '400px', overflow: 'auto' }}>
                        <pre style={{ margin: 0, color: '#d4d4d4', fontFamily: 'monospace', whiteSpace: 'pre-wrap', lineHeight: '1.5' }}>
                            {new TextDecoder().decode(hexData)}
                        </pre>
                    </div>
                )}

                {/* 5. Generic Category Placeholder */}
                {!mime.startsWith('video/') && !mime.startsWith('audio/') && !mime.startsWith('image/') && mime !== 'application/pdf' && !mime.startsWith('text/') && (
                    <div style={{ padding: '60px', textAlign: 'center', color: '#666', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '15px' }}>
                        <div style={{ opacity: 0.3 }}>{getIconForMime()}</div>
                        <div>
                            <p style={{ color: '#aaa', fontWeight: '600' }}>אין תצוגה מקדימה חזותית</p>
                            <p style={{ fontSize: '0.8rem' }}>לחץ על נתוני ה-Hex למטה לחקירה ידנית</p>
                        </div>
                    </div>
                )}
            </div>

            {/* 6. Professional Hex Viewer */}
            <div className="hex-viewer-section" style={{ marginTop: '20px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '10px' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <Shield size={14} style={{ color: '#0078d4' }} />
                        <h4 style={{ margin: 0, color: '#aaa', fontSize: '0.9rem', textTransform: 'uppercase', letterSpacing: '1px' }}>Binary Kernel View (First 4KB)</h4>
                    </div>
                    {loading && <span className="pulse" style={{ fontSize: '0.8rem', color: '#0078d4' }}>קורא סקטורים...</span>}
                </div>

                {error && <div style={{ color: '#ff4d4d', fontSize: '0.8rem', padding: '10px', backgroundColor: 'rgba(255,0,0,0.1)', borderRadius: '4px' }}>{error}</div>}

                <div className="hex-grid" style={{
                    backgroundColor: '#050505',
                    padding: '15px',
                    borderRadius: '4px',
                    border: '1px solid #222',
                    maxHeight: '300px',
                    overflowY: 'auto',
                    fontSize: '12px'
                }}>
                    {hexData && Array.from({ length: Math.ceil(hexData.length / 16) }).map((_: any, i: number) => renderHexLine(i * 16))}
                </div>
            </div>
        </div>
    );
};
