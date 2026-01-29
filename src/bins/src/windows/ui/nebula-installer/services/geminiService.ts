import { GoogleGenAI, Type } from "@google/genai";
import { InstallConfig } from "../types";

const ai = new GoogleGenAI({ apiKey: process.env.API_KEY });

export const getSmartConfigRecommendation = async (
  userDescription: string
): Promise<Partial<InstallConfig>> => {
  try {
    const response = await ai.models.generateContent({
      model: "gemini-3-flash-preview",
      contents: `Analyze the user's description and recommend installation settings for the Nebula App.
      User Description: "${userDescription}"
      
      Nebula App Features:
      - DevTools: For developers debugging plugins.
      - Documentation: Offline PDF guides.
      - Cloud Sync: Syncs workspace across devices (good for multi-device users).
      - High Performance: Uses more RAM/GPU (good for heavy workloads).
      
      Return a JSON object with boolean recommendations.`,
      config: {
        responseMimeType: "application/json",
        responseSchema: {
          type: Type.OBJECT,
          properties: {
            enableDevTools: { type: Type.BOOLEAN },
            installDocumentation: { type: Type.BOOLEAN },
            enableCloudSync: { type: Type.BOOLEAN },
            highPerformanceMode: { type: Type.BOOLEAN },
            reasoning: { type: Type.STRING, description: "Short explanation of choices" }
          },
          required: ["enableDevTools", "installDocumentation", "enableCloudSync", "highPerformanceMode"]
        }
      }
    });

    const text = response.text;
    if (!text) return {};
    
    const data = JSON.parse(text);
    return {
      enableDevTools: data.enableDevTools,
      installDocumentation: data.installDocumentation,
      enableCloudSync: data.enableCloudSync,
      highPerformanceMode: data.highPerformanceMode,
    };
  } catch (error) {
    console.error("Gemini recommendation failed:", error);
    return {};
  }
};