import React, { useState, useEffect } from 'react';
import { ChevronRight, ChevronDown, Folder, File, FileCode } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  children?: FileEntry[];
}

export function FileExplorer({ rootPath }: { rootPath: string }) {
  const [entries, setEntries] = useState<FileEntry[]>([]);

  useEffect(() => {
    if (rootPath) {
      loadFiles(rootPath);
    }
  }, [rootPath]);

  const loadFiles = async (path: string) => {
    try {
      const result = await invoke<FileEntry[]>('list_files', { path });
      setEntries(result);
    } catch (err) {
      console.error("Failed to list files:", err);
    }
  };

  return (
    <div className="flex-1 flex flex-col overflow-y-auto px-2 py-1 select-none">
      {entries.map((entry) => (
        <FileItem key={entry.path} entry={entry} depth={0} />
      ))}
    </div>
  );
}

function FileItem({ entry, depth }: { entry: FileEntry; depth: number }) {
  const [isOpen, setIsOpen] = useState(false);
  const [children, setChildren] = useState<FileEntry[]>([]);

  const toggle = async () => {
    if (entry.is_dir) {
      if (!isOpen && children.length === 0) {
        try {
          const result = await invoke<FileEntry[]>('list_files', { path: entry.path });
          setChildren(result);
        } catch (err) {
          console.error("Failed to list files:", err);
        }
      }
      setIsOpen(!isOpen);
    }
  };

  return (
    <div className="flex flex-col">
      <div 
        onClick={toggle}
        className="flex items-center gap-2 px-2 py-0.5 rounded hover:bg-white/5 cursor-pointer text-text text-xs"
        style={{ paddingLeft: `${depth * 12 + 8}px` }}
      >
        {entry.is_dir && (
          <span className="text-text-muted">
            {isOpen ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          </span>
        )}
        {!entry.is_dir && <span className="w-3" />}
        <span className="text-blue-400">
          {entry.is_dir ? <Folder size={14} /> : entry.name.match(/\.(ts|tsx|js|jsx)$/) ? <FileCode size={14} /> : <File size={14} />}
        </span>
        <span className="truncate">{entry.name}</span>
      </div>
      {isOpen && entry.is_dir && children.length > 0 && (
        <div className="flex flex-col">
          {children.map((child) => (
            <FileItem key={child.path} entry={child} depth={depth + 1} />
          ))}
        </div>
      )}
    </div>
  );
}
