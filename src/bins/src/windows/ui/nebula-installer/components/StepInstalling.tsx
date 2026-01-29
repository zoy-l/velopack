import React, { useEffect, useState, useRef } from 'react';
import { Loader2, Terminal } from 'lucide-react';
import { LogEntry } from '../types';

interface Props {
  onComplete: () => void;
}

export const StepInstalling: React.FC<Props> = ({ onComplete }) => {
  const [progress, setProgress] = useState(0);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const logsEndRef = useRef<HTMLDivElement>(null);

  const tasks = [
    "Initializing package manager...",
    "Verifying system requirements...",
    "Allocating disk space...",
    "Downloading core binaries [145MB]...",
    "Extracting archive...",
    "Optimizing assets...",
    "Configuring registry keys...",
    "Registering services...",
    "Cleaning up temporary files...",
    "Finalizing installation..."
  ];

  useEffect(() => {
    let currentTaskIndex = 0;
    const interval = setInterval(() => {
      if (currentTaskIndex >= tasks.length) {
        clearInterval(interval);
        setTimeout(onComplete, 800);
        return;
      }

      const newTask = tasks[currentTaskIndex];
      setLogs(prev => [
        ...prev,
        {
          id: Date.now(),
          message: newTask,
          timestamp: new Date().toLocaleTimeString(),
          status: 'info'
        }
      ]);

      setProgress(prev => Math.min(prev + (100 / tasks.length), 100));
      currentTaskIndex++;
    }, 600);

    return () => clearInterval(interval);
  }, [onComplete]);

  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  return (
    <div className="flex flex-col h-full justify-center space-y-6 animate-fade-in">
      <div className="text-center space-y-2">
        <div className="inline-flex items-center justify-center w-10 h-10 rounded-full mb-2 bg-gray-100 dark:bg-neutral-800">
            <Loader2 className="w-5 h-5 animate-spin text-black dark:text-white" />
        </div>
        <h2 className="text-lg font-bold text-gray-900 dark:text-white">Installing...</h2>
        <p className="text-xs text-gray-500 dark:text-neutral-400">Please wait while we set up your workspace.</p>
      </div>

      <div className="space-y-1.5">
        <div className="flex justify-between text-xs font-mono text-gray-500 dark:text-neutral-500">
          <span>{logs.length > 0 ? logs[logs.length - 1].message : 'Preparing...'}</span>
          <span>{Math.round(progress)}%</span>
        </div>
        <div className="h-1.5 rounded-full overflow-hidden bg-gray-200 dark:bg-neutral-800">
          <div 
            className="h-full transition-all duration-300 ease-out bg-black dark:bg-white"
            style={{ width: `${progress}%` }}
          />
        </div>
      </div>

      <div className="rounded border font-mono text-[10px] p-3 h-32 overflow-y-auto custom-scrollbar flex flex-col gap-1 transition-colors duration-300 bg-gray-50 border-gray-200 dark:bg-neutral-900 dark:border-neutral-800">
        <div className="flex items-center gap-1.5 mb-1 border-b pb-1 text-gray-500 border-gray-200 dark:text-neutral-500 dark:border-neutral-800">
            <Terminal size={10} />
            <span className="font-bold">INSTALLATION LOG</span>
        </div>
        {logs.map(log => (
          <div key={log.id} className="flex gap-2 text-gray-500 dark:text-neutral-400">
            <span className="shrink-0 select-none text-gray-400 dark:text-neutral-600">[{log.timestamp}]</span>
            <span className={log.status === 'info' ? 'text-gray-600 dark:text-neutral-400' : 'text-yellow-600 dark:text-yellow-500'}>
              {log.message}
            </span>
          </div>
        ))}
        <div ref={logsEndRef} />
      </div>
    </div>
  );
};