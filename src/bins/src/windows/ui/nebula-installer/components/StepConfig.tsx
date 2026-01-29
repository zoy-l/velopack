import React, { useState } from 'react';
import { Settings, Sparkles, Server, Book, Zap, Code } from 'lucide-react';
import { InstallConfig } from '../types';
import { getSmartConfigRecommendation } from '../services/geminiService';

interface Props {
  config: InstallConfig;
  setConfig: React.Dispatch<React.SetStateAction<InstallConfig>>;
  onNext: () => void;
  onBack: () => void;
}

export const StepConfig: React.FC<Props> = ({ config, setConfig, onNext, onBack }) => {
  const [userPrompt, setUserPrompt] = useState('');
  const [isAnalyzing, setIsAnalyzing] = useState(false);

  const handleSmartConfig = async () => {
    if (!userPrompt.trim()) return;
    setIsAnalyzing(true);
    try {
      const recommendations = await getSmartConfigRecommendation(userPrompt);
      setConfig(prev => ({ ...prev, ...recommendations }));
    } catch (e) {
      console.error(e);
    } finally {
      setIsAnalyzing(false);
    }
  };

  const toggle = (key: keyof InstallConfig) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    setConfig(prev => ({ ...prev, [key]: !prev[key as any] }));
  };

  return (
    <div className="flex flex-col h-full max-w-4xl mx-auto space-y-6 animate-fade-in">
      <div className="flex items-center justify-between border-b border-white/10 pb-4">
        <div className="flex items-center gap-3">
          <Settings className="w-6 h-6 text-indigo-400" />
          <h2 className="text-2xl font-semibold text-white">Custom Installation</h2>
        </div>
      </div>

      {/* AI Assistant Section */}
      <div className="bg-gradient-to-r from-indigo-900/30 to-purple-900/30 rounded-xl p-1 border border-white/10">
        <div className="bg-slate-900/80 rounded-lg p-4 backdrop-blur-sm">
          <div className="flex items-start gap-4">
            <div className="p-2 bg-indigo-500/20 rounded-lg">
              <Sparkles className="w-5 h-5 text-indigo-400" />
            </div>
            <div className="flex-1 space-y-3">
              <div>
                <h3 className="text-white font-medium">Smart Configure</h3>
                <p className="text-slate-400 text-sm">Tell Nebula how you plan to use it, and we'll select the best modules for you.</p>
              </div>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={userPrompt}
                  onChange={(e) => setUserPrompt(e.target.value)}
                  placeholder="e.g. I am a data scientist needing high performance..."
                  className="flex-1 bg-slate-950 border border-slate-700 rounded-lg px-4 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-indigo-500 transition-colors"
                />
                <button
                  onClick={handleSmartConfig}
                  disabled={isAnalyzing || !userPrompt}
                  className="bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed text-white px-4 py-2 rounded-lg text-sm font-medium transition-colors flex items-center gap-2"
                >
                  {isAnalyzing ? 'Analyzing...' : 'Auto-Select'}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Options Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 flex-1 overflow-y-auto p-1">
        <OptionCard
          icon={<Code />}
          title="Developer Tools"
          description="Include debugging symbols, SDKs, and CLI tools."
          active={config.enableDevTools}
          onClick={() => toggle('enableDevTools')}
        />
        <OptionCard
          icon={<Book />}
          title="Documentation"
          description="Install offline PDF guides and API references."
          active={config.installDocumentation}
          onClick={() => toggle('installDocumentation')}
        />
        <OptionCard
          icon={<Server />}
          title="Cloud Sync Agent"
          description="Background service to sync projects with Nebula Cloud."
          active={config.enableCloudSync}
          onClick={() => toggle('enableCloudSync')}
        />
        <OptionCard
          icon={<Zap />}
          title="High Performance"
          description="Optimize memory allocation for large projects."
          active={config.highPerformanceMode}
          onClick={() => toggle('highPerformanceMode')}
        />
      </div>

      <div className="flex justify-between pt-4 border-t border-white/5">
        <button onClick={onBack} className="px-6 py-2.5 rounded-lg text-slate-400 hover:text-white hover:bg-white/5 transition-colors">
          Back
        </button>
        <button
          onClick={onNext}
          className="px-8 py-2.5 rounded-lg font-medium bg-indigo-600 text-white hover:bg-indigo-500 shadow-lg shadow-indigo-500/20 transition-all"
        >
          Install Now
        </button>
      </div>
    </div>
  );
};

const OptionCard: React.FC<{
  icon: React.ReactNode;
  title: string;
  description: string;
  active: boolean;
  onClick: () => void;
}> = ({ icon, title, description, active, onClick }) => (
  <div
    onClick={onClick}
    className={`p-4 rounded-xl border cursor-pointer transition-all duration-200 group relative overflow-hidden ${
      active
        ? 'bg-indigo-600/10 border-indigo-500/50 hover:bg-indigo-600/20'
        : 'bg-slate-800/30 border-white/5 hover:bg-slate-800/50 hover:border-white/10'
    }`}
  >
    <div className="flex items-start gap-4 relative z-10">
      <div className={`p-2 rounded-lg transition-colors ${active ? 'bg-indigo-500 text-white' : 'bg-slate-700/50 text-slate-400 group-hover:text-slate-200'}`}>
        {React.cloneElement(icon as React.ReactElement, { size: 20 })}
      </div>
      <div>
        <h3 className={`font-medium transition-colors ${active ? 'text-white' : 'text-slate-300 group-hover:text-white'}`}>
          {title}
        </h3>
        <p className="text-sm text-slate-500 mt-1 leading-relaxed">
          {description}
        </p>
      </div>
    </div>
    {/* Active indicator glow */}
    {active && <div className="absolute top-0 right-0 w-16 h-16 bg-indigo-500/20 blur-2xl -mr-8 -mt-8 pointer-events-none"></div>}
  </div>
);