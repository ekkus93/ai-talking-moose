import React from "react";
import { SettingsModal as SettingsModalBase } from "./SettingsModalBase";
import { P6VoiceAcceptancePanel } from "./P6VoiceAcceptancePanel";

export const SettingsModal: React.FC = () => (
  <>
    <SettingsModalBase />
    <P6VoiceAcceptancePanel />
  </>
);
