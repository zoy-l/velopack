import React from 'react';
import { Package, ArrowRight } from 'lucide-react';

interface Props {
  onNext: () => void;
}

export const StepWelcome: React.FC<Props> = ({ onNext }) => {
  return (
    <div className="flex flex-col h-full justify-center items-center text-center space-y-6 animate-fade-in">
      <div className="relative">
        <div className="w-16 h-16 rounded-2xl flex items-center justify-center border transition-colors duration-300 bg-gray-50 dark:bg-neutral-900 border-gray-200 dark:border-neutral-800">
             <Package className="w-8 h-8 text-black dark:text-white" strokeWidth={1.5} />
        </div>
      </div>
      
      <div className="space-y-2 max-w-sm">
        <h1 className="text-xl font-bold text-gray-900 dark:text-white tracking-tight">
          Welcome to Nebula
        </h1>
        <p className="text-gray-500 dark:text-neutral-400 text-sm">
          The next generation workspace for creative minds. 
          Click below to begin the setup.
        </p>
      </div>

      <div className="pt-4">
        <button
          onClick={onNext}
          className="group relative inline-flex items-center gap-2 px-6 py-2 rounded-lg font-medium text-sm transition-all duration-200 bg-black text-white hover:bg-gray-800 dark:bg-white dark:text-black dark:hover:bg-gray-200"
        >
          Get Started
          <ArrowRight className="w-4 h-4 group-hover:translate-x-0.5 transition-transform" />
        </button>
      </div>
    </div>
  );
};