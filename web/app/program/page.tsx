// app/program/page.tsx
import { fetchSchedule } from "@/app/actions/schedule-actions";
import ProgramTable from "./ProgramTable";
import NewProgramModal from "./NewProgramModal";

export default async function ProgramPage() {
  const schedule = await fetchSchedule();

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Programok (v{schedule?.version ?? 0})</h1>
        <NewProgramModal />
      </div>
      {schedule?.programs?.length ? (
        <ProgramTable programs={schedule.programs} />
      ) : (
        <p>Nincs még program.</p>
      )}
    </div>
  );
}