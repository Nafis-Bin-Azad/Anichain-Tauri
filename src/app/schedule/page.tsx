"use client";

import { useEffect, useState } from "react";
import { tauri } from "@/lib/tauri";
import { Loader2 } from "lucide-react";

interface ScheduleEntry {
  title: string;
  episode: string;
  air_date: string;
}

export default function Schedule() {
  const [schedule, setSchedule] = useState<ScheduleEntry[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const loadSchedule = async () => {
      try {
        const data = await tauri.invoke<ScheduleEntry[]>("get_schedule");
        setSchedule(data);
      } catch (error) {
        console.error("Failed to load schedule:", error);
      } finally {
        setLoading(false);
      }
    };

    loadSchedule();
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[calc(100vh-8rem)]">
        <Loader2 className="w-8 h-8 animate-spin text-blue-500" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900">Schedule</h1>
        <p className="text-sm text-gray-500">
          {schedule.length} upcoming episodes
        </p>
      </div>
      <div className="space-y-4">
        {schedule.map((entry) => (
          <div
            key={`${entry.title}-${entry.episode}`}
            className="bg-white p-4 rounded-lg shadow-sm"
          >
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-lg font-semibold text-gray-900">
                  {entry.title}
                </h3>
                <p className="text-sm text-gray-600">Episode {entry.episode}</p>
              </div>
              <div className="text-sm text-gray-500">{entry.air_date}</div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
