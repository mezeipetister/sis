'use server';
import {DateTime} from 'ts-luxon';


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

  const schedule: Schedule = await res.json();
  schedule.programs.forEach(program => {
    // Convert start_time from UTC to Europe/Budapest local time (HH:mm)
    const [hour, minute] = program.start_time.split(':').map(Number);
    const utcTime =
        DateTime.utc().set({hour, minute, second: 0, millisecond: 0});
    const budapestTime = utcTime.setZone('Europe/Budapest');
    const localHour = budapestTime.hour.toString().padStart(2, '0');
    const localMinute = budapestTime.minute.toString().padStart(2, '0');
    program.start_time = `${localHour}:${localMinute}`;
  });

  return schedule;
}

export async function setProgram(program: Program): Promise<boolean> {
  if (program.start_time) {
    const [hour, minute] = program.start_time.split(':').map(Number);

    // Készítsünk egy időpontot a mai napra, Europe/Budapest zónában
    const localTime = DateTime.local()
                          .setZone('Europe/Budapest')
                          .set({hour, minute, second: 0, millisecond: 0});

    // Konvertáljuk UTC-re
    const utcTime = localTime.toUTC();

    // Vegyük ki az órát és percet, és formázzuk HH:mm formátumba
    const utcHour = utcTime.hour.toString().padStart(2, '0');
    const utcMinute = utcTime.minute.toString().padStart(2, '0');

    program.start_time = `${utcHour}:${utcMinute}`;
  }

  console.log('Converted start_time to UTC:', program.start_time);

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