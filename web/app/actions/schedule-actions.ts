'use server';

export type ZoneAction = {
  zone_ids: string[]; duration_seconds: number;
};

export type Program = {
  id: string; name: string; weekdays: number[]; start_time: string;
  active: boolean;
  zones: ZoneAction[];
};

export type Schedule = {
  version: number; programs: Program[];
};

export type ClientCommand =|{
  type: 'StartProgram';
  program_id: string
}
|{
  type: 'StartZoneAction';
  zone_action: ZoneAction
}
|{type: 'Stop'};

// API base URL
const API_BASE = 'http://server:3400';

export async function fetchSchedule(): Promise<Schedule|null> {
  const res = await fetch(`${API_BASE}/schedule`, {cache: 'no-store'});
  if (!res.ok) return null;
  return await res.json();
}

export async function setProgram(program: Program): Promise<boolean> {
  const res = await fetch(`${API_BASE}/schedule/program`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify(program),
  });
  return res.ok;
}

export async function enableProgram(id: string): Promise<boolean> {
  const res = await fetch(`${API_BASE}/schedule/program/${id}/enable`, {
    method: 'POST',
  });
  return res.ok;
}

export async function disableProgram(id: string): Promise<boolean> {
  const res = await fetch(`${API_BASE}/schedule/program/${id}/disable`, {
    method: 'POST',
  });
  return res.ok;
}

export async function removeProgram(id: string): Promise<boolean> {
  const res = await fetch(`${API_BASE}/schedule/program/${id}/remove`, {
    method: 'POST',
  });
  return res.ok;
}

export async function sendClientCommand(command: ClientCommand):
    Promise<boolean> {
  const res = await fetch(`${API_BASE}/run_command`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify(command),
  });
  return res.ok;
}