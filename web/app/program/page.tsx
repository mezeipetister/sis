// app/program/page.tsx
import { fetchSchedule } from "@/app/actions/schedule-actions";
import ProgramTable from "./ProgramTable";
import NewProgramModal from "./NewProgramModal";
import { getZonesAll } from "../actions/board-actions";

export default async function ProgramPage() {
  const schedule = await fetchSchedule();
  const zones = await getZonesAll();


  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Programok (v{schedule?.version ?? 0})</h1>
        <NewProgramModal />
      </div>
      {schedule?.programs?.length ? (
        <ProgramTable programs={schedule.programs} zones={zones} />
      ) : (
        <p>Nincs még program.</p>
      )}
    </div>
  );
}