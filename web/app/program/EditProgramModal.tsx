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

	const initialSelectedZones = program.zones.map((pz: any) => ({
		zone_ids: [...pz.zone_ids],
		duration_seconds: pz.duration_seconds,
	}));

	const [selectedZones, setSelectedZones] = useState(initialSelectedZones);

	const toggleWeekday = (day: number) => {
		setWeekdays((prev) =>
			prev.includes(day) ? prev.filter((d) => d !== day) : [...prev, day]
		);
	};

	const handleZoneToggle = (zoneId: string) => {
		setSelectedZones((prev) => {
			const exists = prev.find((z) => z.zone_ids.includes(zoneId));
			if (exists) {
				return prev.filter((z) => !z.zone_ids.includes(zoneId));
			} else {
				return [...prev, { zone_ids: [zoneId], duration_seconds: 300 }];
			}
		});
	};

	const handleSubmit = async () => {
		await setProgram({
			id: program.id,
			name,
			start_time: startTime,
			weekdays,
			active: program.active,
			zones: selectedZones,
		});
		onClose();
		location.reload();
	};

	const handleDragEnd = (result: DropResult) => {
		if (!result.destination) return;
		const reordered = Array.from(selectedZones);
		const [removed] = reordered.splice(result.source.index, 1);
		reordered.splice(result.destination.index, 0, removed);
		setSelectedZones(reordered);
	};

	return (
		<div className="fixed inset-0 bg-black bg-opacity-40 flex items-center justify-center z-50">
			<div className="bg-white p-6 rounded shadow max-w-md w-full">
				<h2 className="text-lg font-bold mb-4">Program szerkesztése</h2>
				<input
					placeholder="Név"
					className="border px-3 py-2 mb-2 w-full"
					value={name}
					onChange={(e) => setName(e.target.value)}
				/>
				<input
					type="time"
					className="border px-3 py-2 mb-2 w-full"
					value={startTime}
					onChange={(e) => setStartTime(e.target.value)}
				/>
				<div className="mb-4">
					<label className="block font-medium mb-1">Napok:</label>
					{[0, 1, 2, 3, 4, 5, 6].map((d) => (
						<button
							key={d}
							onClick={() => toggleWeekday(d)}
							className={`px-2 py-1 border rounded mr-1 mb-1 ${weekdays.includes(d) ? "bg-blue-500 text-white" : "bg-white"}`}
						>
							{"VHSCPPS"[d]}
						</button>
					))}
				</div>
				<div className="mb-4">
					<label className="block font-medium mb-1">Program zónái és időtartamuk (mp):</label>

					<DragDropContext onDragEnd={handleDragEnd}>
						<Droppable droppableId="zones-list">
							{(provided) => (
								<div
									className="flex flex-col gap-2"
									ref={provided.innerRef}
									{...provided.droppableProps}
								>
									{selectedZones.map((sz, idx) => {
										const zone = zones.find((z) => sz.zone_ids.includes(z.id));
										return (
											<Draggable key={sz.zone_ids[0]} draggableId={sz.zone_ids[0]} index={idx}>
												{(provided, snapshot) => (
													<div
														ref={provided.innerRef}
														{...provided.draggableProps}
														{...provided.dragHandleProps}
														className={`flex items-center gap-3 bg-white ${snapshot.isDragging ? "shadow-lg" : ""}`}
													>
														<span className="cursor-move px-2 py-1 rounded">:::</span>
														<span className="min-w-[100px]">{zone?.name || sz.zone_ids[0]}</span>
														<input
															type="number"
															min={1}
															className="border px-2 py-1 w-24"
															value={Math.floor(sz.duration_seconds / 60)}
															onChange={(e) => {
																const value = Number(e.target.value);
																setSelectedZones((prev) =>
																	prev.map((z, i) =>
																		i === idx
																			? { ...z, duration_seconds: value * 60 }
																			: z
																	)
																);
															}}
														/>
														<span>perc</span>
													</div>
												)}
											</Draggable>
										);
									})}
									{provided.placeholder}
								</div>
							)}
						</Droppable>
					</DragDropContext>
				</div>
				<div className="mb-4">
					<label className="block font-medium mb-1">Kiválasztott zónák:</label>
					<div className="flex flex-col gap-1">
						{zones.map((zone) => (
							<label key={zone.id} className="flex items-center gap-2">
								<input
									type="checkbox"
									checked={selectedZones.some((z) => z.zone_ids.includes(zone.id))}
									onChange={() => handleZoneToggle(zone.id)}
								/>
								{zone.name || zone.id}
							</label>
						))}
					</div>
				</div>
				<button
					onClick={handleSubmit}
					className="bg-green-600 text-white px-4 py-2 rounded"
				>
					Mentés
				</button>
				<button onClick={onClose} className="ml-2 text-gray-600">
					Mégse
				</button>
			</div>
		</div>
	);
}