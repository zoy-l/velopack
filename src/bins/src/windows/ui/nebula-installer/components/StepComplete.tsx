import React from 'react';
import { Check, Rocket, Github, Twitter } from 'lucide-react';

export const StepComplete: React.FC = () => {
  return (
    <div className="flex flex-col h-full justify-center items-center text-center space-y-6 animate-fade-in">
      <div className="relative">
        <div className="w-16 h-16 rounded-full flex items-center justify-center border transition-colors bg-emerald-50 border-emerald-100 dark:bg-emerald-950/30 dark:border-emerald-900/50">
          <Check className="w-8 h-8 text-emerald-600 dark:text-emerald-500" strokeWidth={3} />
        </div>
      </div>
      
      <div className="space-y-2 max-w-sm">
        <h1 className="text-xl font-bold tracking-tight text-gray-900 dark:text-white">
          Installation Complete!
        </h1>
        <p className="text-sm text-gray-500 dark:text-neutral-400">
          Nebula has been successfully installed. You are ready to build the future.
        </p>
      </div>

      <div className="pt-4 w-full max-w-xs space-y-2.5">
        <button
          className="w-full flex items-center justify-center gap-2 px-6 py-2 rounded-lg font-bold text-sm transition-all duration-200 shadow-sm bg-black text-white hover:bg-gray-800 dark:bg-white dark:text-black dark:hover:bg-gray-200"
          onClick={() => window.alert("Launching Nebula App...")}
        >
          <Rocket className="w-4 h-4" />
          Launch Nebula
        </button>
        
        <button
          className="w-full flex items-center justify-center gap-2 px-6 py-2 rounded-lg font-medium text-sm transition-all bg-gray-100 text-gray-600 hover:bg-gray-200 dark:bg-neutral-800 dark:text-neutral-300 dark:hover:bg-neutral-700"
          onClick={() => window.location.reload()}
        >
          Exit Installer
        </button>
      </div>
      
      <div className="flex gap-4 pt-4 text-gray-400 dark:text-neutral-600">
        <a href="#" className="hover:text-black dark:hover:text-white transition-colors"><Github className="w-4 h-4"/></a>
        <a href="#" className="hover:text-black dark:hover:text-white transition-colors"><Twitter className="w-4 h-4"/></a>
      </div>
    </div>
  );
};