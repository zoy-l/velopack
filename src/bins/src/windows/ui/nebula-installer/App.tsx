import React, { useState, useEffect } from 'react';
import { InstallStep } from './types';
import { StepWelcome } from './components/StepWelcome';
import { StepLicense } from './components/StepLicense';
import { StepInstalling } from './components/StepInstalling';
import { StepComplete } from './components/StepComplete';
import { Disc, Shield, HardDrive, CheckCircle, Moon, Sun, X } from 'lucide-react';

const App: React.FC = () => {
  const [currentStep, setCurrentStep] = useState<InstallStep>(InstallStep.WELCOME);
  const [isDarkMode, setIsDarkMode] = useState(true);

  // Apply dark mode class to html element
  useEffect(() => {
    if (isDarkMode) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, [isDarkMode]);

  const nextStep = () => {
    const steps = Object.values(InstallStep);
    const currentIndex = steps.indexOf(currentStep);
    if (currentIndex < steps.length - 1) {
      setCurrentStep(steps[currentIndex + 1]);
    }
  };

  const prevStep = () => {
    const steps = Object.values(InstallStep);
    const currentIndex = steps.indexOf(currentStep);
    if (currentIndex > 0) {
      setCurrentStep(steps[currentIndex - 1]);
    }
  };

  const renderStep = () => {
    switch (currentStep) {
      case InstallStep.WELCOME:
        return <StepWelcome onNext={nextStep} />;
      case InstallStep.LICENSE:
        return <StepLicense onNext={nextStep} onBack={prevStep} />;
      case InstallStep.INSTALLING:
        return <StepInstalling onComplete={nextStep} />;
      case InstallStep.COMPLETE:
        return <StepComplete />;
      default:
        return <div>Unknown Step</div>;
    }
  };

  const stepsList = [
    { id: InstallStep.WELCOME, label: 'Welcome', icon: Disc },
    { id: InstallStep.LICENSE, label: 'License', icon: Shield },
    { id: InstallStep.INSTALLING, label: 'Install', icon: HardDrive },
    { id: InstallStep.COMPLETE, label: 'Finish', icon: CheckCircle },
  ];

  return (
    <div className={`min-h-screen flex items-center justify-center p-4 transition-colors duration-300 ${isDarkMode ? 'bg-neutral-900' : 'bg-gray-100'}`}>
        
      {/* Main Installer Window */}
      <div className={`w-full max-w-2xl h-[480px] border rounded-xl shadow-2xl flex overflow-hidden relative z-10 transition-colors duration-300 
          ${isDarkMode ? 'bg-neutral-950 border-neutral-800 text-neutral-200' : 'bg-white border-gray-200 text-gray-900'}
      `}>
        
        {/* Sidebar */}
        <div className={`w-44 border-r flex flex-col p-4 hidden md:flex transition-colors duration-300
            ${isDarkMode ? 'bg-neutral-900/50 border-neutral-800' : 'bg-gray-50 border-gray-100'}
        `}>
          <div className="mb-6 flex items-center gap-2.5">
             <div className={`w-6 h-6 rounded-md flex items-center justify-center shadow-sm transition-colors
                 ${isDarkMode ? 'bg-white text-black' : 'bg-black text-white'}
             `}>
                <span className="font-bold text-xs">N</span>
             </div>
             <span className={`text-sm font-bold tracking-wide ${isDarkMode ? 'text-white' : 'text-black'}`}>Nebula</span>
          </div>

          <div className="space-y-1 relative">
            {/* Connecting Line */}
            <div className={`absolute left-[15px] top-3 bottom-3 w-[2px] transition-colors ${isDarkMode ? 'bg-neutral-800' : 'bg-gray-200'}`}></div>

            {stepsList.map((step, index) => {
               const isActive = step.id === currentStep;
               const isCompleted = Object.values(InstallStep).indexOf(step.id) < Object.values(InstallStep).indexOf(currentStep);
               const Icon = step.icon;

               return (
                <div key={step.id} className="relative flex items-center gap-3 py-2 z-10 group">
                    <div className={`w-8 h-8 rounded-full flex items-center justify-center border transition-all duration-200 
                        ${isActive 
                            ? (isDarkMode ? 'bg-white border-white text-black' : 'bg-black border-black text-white') 
                            : isCompleted 
                                ? (isDarkMode ? 'bg-neutral-900 border-neutral-700 text-neutral-500' : 'bg-white border-gray-300 text-gray-400') 
                                : (isDarkMode ? 'bg-neutral-950 border-neutral-800 text-neutral-600' : 'bg-white border-gray-200 text-gray-300')
                        }
                    `}>
                        <Icon size={14} />
                    </div>
                    <div className="flex flex-col">
                        <span className={`font-medium text-xs transition-colors 
                            ${isActive 
                                ? (isDarkMode ? 'text-white' : 'text-black') 
                                : isCompleted 
                                    ? (isDarkMode ? 'text-neutral-500' : 'text-gray-400') 
                                    : (isDarkMode ? 'text-neutral-600' : 'text-gray-300')
                            }
                        `}>
                            {step.label}
                        </span>
                    </div>
                </div>
               );
            })}
          </div>

          <div className={`mt-auto pt-4 border-t transition-colors ${isDarkMode ? 'border-neutral-800' : 'border-gray-200'}`}>
            <div className="flex items-center gap-2">
                <div className="w-1.5 h-1.5 rounded-full bg-emerald-500"></div>
                <span className={`text-[10px] font-mono ${isDarkMode ? 'text-neutral-500' : 'text-gray-400'}`}>v2.0.4 stable</span>
            </div>
          </div>
        </div>

        {/* Content Area */}
        <div className={`flex-1 relative flex flex-col transition-colors duration-300 ${isDarkMode ? 'bg-neutral-950' : 'bg-white'}`}>
            {/* Top Bar with Controls */}
            <div className={`h-10 border-b flex items-center justify-between pl-4 pr-2 transition-colors duration-300 select-none ${isDarkMode ? 'border-neutral-800' : 'border-gray-100'}`}>
                 <button 
                    onClick={() => setIsDarkMode(!isDarkMode)}
                    className={`p-1.5 rounded-lg transition-colors ${isDarkMode ? 'hover:bg-neutral-800 text-neutral-400' : 'hover:bg-gray-100 text-gray-400'}`}
                 >
                    {isDarkMode ? <Sun size={14} /> : <Moon size={14} />}
                 </button>

                 <button 
                    className={`p-1.5 rounded-md transition-colors 
                        ${isDarkMode 
                            ? 'text-neutral-400 hover:bg-red-600 hover:text-white' 
                            : 'text-gray-400 hover:bg-red-500 hover:text-white'
                        }
                    `}
                    title="Close"
                 >
                    <X size={18} strokeWidth={2} />
                 </button>
            </div>

            {/* Step Content */}
            <div className="flex-1 p-6 overflow-y-auto relative">
                {renderStep()}
            </div>
        </div>
      </div>
    </div>
  );
};

export default App;