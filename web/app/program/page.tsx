
import { Schedule } from "@/app/actions/schedule-actions";

export default async function SchedulePage() {
  // API base URL
  const API_BASE = 'http://server:3400';

  async function fetchSchedule(): Promise<Schedule | null> {
    const res = await fetch(`${API_BASE}/schedule`, { cache: 'no-store' });
    if (!res.ok) return null;
    return await res.json();
  }

  const schedule = await fetchSchedule();

  if (!schedule) return <p>Hiba történt az adatok betöltésekor.</p>;

  return (
    <div>
      <h1 className="text-xl font-bold">Öntözési programok (verzió: {schedule.version})</h1>
      <ul className="mt-4 space-y-2">
        {schedule.programs.map((p) => (
          <li key={p.id}>
            <b>{p.name}</b> ({p.start_time}) – {p.active ? "Aktív" : "Inaktív"}
          </li>
        ))}
      </ul>
    </div>
  );
}