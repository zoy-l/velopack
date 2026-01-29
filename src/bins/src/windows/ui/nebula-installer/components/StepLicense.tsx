import React, { useState } from 'react';
import { ShieldCheck, ScrollText } from 'lucide-react';

interface Props {
  onNext: () => void;
  onBack: () => void;
}

export const StepLicense: React.FC<Props> = ({ onNext, onBack }) => {
  const [accepted, setAccepted] = useState(false);

  return (
    <div className="flex flex-col h-full space-y-4 animate-fade-in">
      <div className="flex items-center gap-2 border-b pb-3 border-gray-100 dark:border-neutral-800">
        <ScrollText className="w-5 h-5 text-black dark:text-white" />
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white">License Agreement</h2>
      </div>

      <div className="flex-1 rounded-lg border p-4 overflow-y-auto custom-scrollbar shadow-inner text-xs leading-relaxed transition-colors duration-300 bg-gray-50 border-gray-200 text-gray-600 dark:bg-neutral-900 dark:border-neutral-800 dark:text-neutral-400">
        <h3 className="font-bold mb-2 text-gray-900 dark:text-white">NEBULA END USER LICENSE AGREEMENT</h3>
        <p className="mb-3">
          IMPORTANT: PLEASE READ THIS LICENSE CAREFULLY BEFORE USING THIS SOFTWARE.
        </p>
        <p className="mb-3">
          1. <strong>LICENSE GRANT</strong>
          <br />
          "Nebula" (the "Software") is licensed, not sold. We grant you a revocable, non-exclusive, non-transferable, limited license to download, install and use the Software solely for your personal, non-commercial purposes strictly in accordance with the terms of this Agreement.
        </p>
        <p className="mb-3">
          2. <strong>RESTRICTIONS</strong>
          <br />
          You agree not to, and you will not permit others to: license, sell, rent, lease, assign, distribute, transmit, host, outsource, disclose or otherwise commercially exploit the Software.
        </p>
        <p className="mb-3">
          3. <strong>TERMINATION</strong>
          <br />
          This Agreement shall remain in effect until terminated by you or us. We may, in our sole discretion, at any time and for any or no reason, suspend or terminate this Agreement with or without prior notice.
        </p>
        <p>
          By clicking "I Accept", you acknowledge that you have read this agreement.
        </p>
      </div>

      <div className="flex items-center gap-2 p-3 rounded-lg border cursor-pointer transition-colors duration-200 bg-white border-gray-200 hover:bg-gray-50 dark:bg-neutral-900 dark:border-neutral-800 dark:hover:bg-neutral-800" onClick={() => setAccepted(!accepted)}>
        <div className={`w-4 h-4 rounded border flex items-center justify-center transition-colors 
            ${accepted 
                ? 'bg-black border-black dark:bg-white dark:border-white' 
                : 'bg-transparent border-gray-400 dark:border-neutral-600'
            }`}>
          {accepted && <ShieldCheck className="w-3 h-3 text-white dark:text-black" />}
        </div>
        <span className="text-xs select-none text-gray-600 dark:text-neutral-300">I accept the terms and conditions outlined above.</span>
      </div>

      <div className="flex justify-between pt-2">
        <button onClick={onBack} className="px-4 py-2 rounded-lg text-sm transition-colors text-gray-500 hover:text-gray-900 hover:bg-gray-100 dark:text-neutral-500 dark:hover:text-white dark:hover:bg-neutral-800">
          Back
        </button>
        <button 
          onClick={onNext} 
          disabled={!accepted}
          className={`px-6 py-2 rounded-lg text-sm font-medium transition-all shadow-sm
            ${accepted 
                ? 'bg-black text-white hover:bg-gray-800 dark:bg-white dark:text-black dark:hover:bg-gray-200' 
                : 'bg-gray-200 text-gray-400 dark:bg-neutral-800 dark:text-neutral-600 cursor-not-allowed'
            }`}
        >
          Next
        </button>
      </div>
    </div>
  );
};