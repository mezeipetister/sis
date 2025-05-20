"use client";

import { useState } from "react";
import { Program, setProgram } from "@/app/actions/schedule-actions";
import { ZoneInfo } from "../actions/board-actions";
import { DragDropContext, Droppable, Draggable, DropResult } from '@hello-pangea/dnd';

type Props = {
	program: Program;
	zones: ZoneInfo[];
	onClose: () => void;
};

export default function EditProgramModal({ program, zones, onClose }: Props) {
	const [name, setName] = useState(program.name);
	const [startTime, setStartTime] = useState(program.start_time);
	const [weekdays, setWeekdays] = useState<number[]>(program.weekdays);
	const [zoneActions, setZoneActions] = useState([...program.zones]);
	const [selectedZoneIds, setSelectedZoneIds] = useState<string[]>([]);

	const toggleWeekday = (day: number) => {
		setWeekdays((prev) =>
			prev.includes(day) ? prev.filter((d) => d !== day) : [...prev, day]
		);
	};

	const toggleZoneSelection = (id: string) => {
		setSelectedZoneIds((prev) =>
			prev.includes(id) ? prev.filter((z) => z !== id) : [...prev, id]
		);
	};

	const addZoneAction = () => {
		if (selectedZoneIds.length === 0) return;
		setZoneActions((prev) => [
			...prev,
			{ zone_ids: [...selectedZoneIds], duration_seconds: 60 },
		]);
		setSelectedZoneIds([]);
	};

	const removeZoneAction = (index: number) => {
		setZoneActions((prev) => prev.filter((_, i) => i !== index));
	};

	const updateZoneDuration = (index: number, minutes: number) => {
		setZoneActions((prev) =>
			prev.map((z, i) =>
				i === index ? { ...z, duration_seconds: minutes * 60 } : z
			)
		);
	};

	const handleDragEnd = (result: DropResult) => {
		if (!result.destination) return;
		const reordered = Array.from(zoneActions);
		const [removed] = reordered.splice(result.source.index, 1);
		reordered.splice(result.destination.index, 0, removed);
		setZoneActions(reordered);
	};

	const handleSubmit = async () => {
		await setProgram({
			id: program.id,
			name,
			start_time: startTime,
			weekdays,
			active: program.active,
			zones: zoneActions,
		});
		onClose();
		location.reload();
	};

	return (
		<div className="fixed inset-0 bg-black bg-opacity-40 flex items-center justify-center z-50">
			<div className="bg-white p-6 rounded shadow max-w-md w-full space-y-4">
				<h2 className="text-lg font-bold">Program szerkesztése</h2>

				<input className="border px-3 py-2 w-full" value={name} onChange={(e) => setName(e.target.value)} placeholder="Program neve" />
				<input type="time" className="border px-3 py-2 w-full" value={startTime} onChange={(e) => setStartTime(e.target.value)} />

				<div>
					<label className="block font-medium mb-1">Napok:</label>
					{[1, 2, 3, 4, 5, 6, 7].map((d) => (
						<span key={d} onClick={() => toggleWeekday(d)} className={`px-2 py-1 border rounded mr-1 mb-1 cursor-pointer select-none ${weekdays.includes(d) ? "bg-blue-500 text-white" : "bg-white"}`}>
							{["Hé", "Ke", "Sze", "Csü", "Pé", "Sz", "Va"][d - 1]}
						</span>
					))}
				</div>

				<div>
					<label className="block font-medium mb-1">Programok (csoportok, rendezhetők):</label>
					<DragDropContext onDragEnd={handleDragEnd}>
						<Droppable droppableId="zoneActionsList">
							{(provided) => (
								<div {...provided.droppableProps} ref={provided.innerRef} className="space-y-1">
									{zoneActions.map((za, index) => (
										<Draggable key={index} draggableId={`zoneaction-${index}`} index={index}>
											{(provided, snapshot) => (
												<div
													ref={provided.innerRef}
													{...provided.draggableProps}
													{...provided.dragHandleProps}
													className={`flex items-center gap-2 border p-2 rounded bg-white ${snapshot.isDragging ? "shadow-md" : ""}`}
												>
													<span className="cursor-move text-gray-500">:::</span>
													<span className="flex-1 text-sm">
														{za.zone_ids.map((id) => zones.find((z) => z.id === id)?.name || id).join(", ")}
													</span>
													<input
														type="number"
														min={1}
														value={Math.floor(za.duration_seconds / 60)}
														onChange={(e) => updateZoneDuration(index, Number(e.target.value))}
														className="border px-2 py-1 w-16"
													/>
													<span>perc</span>
													<button onClick={() => removeZoneAction(index)} className="text-red-600 ml-2 font-bold">
														×
													</button>
												</div>
											)}
										</Draggable>
									))}
									{provided.placeholder}
								</div>
							)}
						</Droppable>
					</DragDropContext>
				</div>

				<div>
					<label className="block font-medium mb-1">Zónák kiválasztása új csoporthoz:</label>
					<div className="grid grid-cols-2 gap-2">
						{zones.map((zone) => (
							<label key={zone.id} className="flex items-center gap-2">
								<input
									type="checkbox"
									checked={selectedZoneIds.includes(zone.id)}
									onChange={() => toggleZoneSelection(zone.id)}
								/>
								{zone.name || zone.id}
							</label>
						))}
					</div>
					<button onClick={addZoneAction} className="mt-2 bg-blue-600 text-white px-3 py-1 rounded">
						Hozzáadás
					</button>
				</div>

				<div className="flex justify-end gap-2">
					<button onClick={handleSubmit} className="bg-green-600 text-white px-4 py-2 rounded">
						Mentés
					</button>
					<button onClick={onClose} className="text-gray-600">Mégse</button>
				</div>
			</div>
		</div>
	);
}
