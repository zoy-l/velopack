export enum InstallStep {
  WELCOME = 'WELCOME',
  LICENSE = 'LICENSE',
  INSTALLING = 'INSTALLING',
  COMPLETE = 'COMPLETE',
}

export interface InstallConfig {
  enableDevTools: boolean;
  installDocumentation: boolean;
  enableCloudSync: boolean;
  highPerformanceMode: boolean;
  theme: 'dark' | 'light';
}

export interface LogEntry {
  id: number;
  message: string;
  timestamp: string;
  status: 'info' | 'success' | 'warning';
}