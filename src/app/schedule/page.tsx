"use client";

import { useEffect, useState } from "react";
import { invokeTauri } from "@/lib/tauri";
import { Loader2, AlertCircle } from "lucide-react";
import { useToast } from "@/contexts/ToastContext";

interface ScheduleEntry {
  title: string;
  episode: string;
  air_date: string;
}

export default function Schedule() {
  const [schedule, setSchedule] = useState<ScheduleEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { showToast } = useToast();

  useEffect(() => {
    const loadSchedule = async () => {
      try {
        console.log("Loading schedule data...");
        const data = await invokeTauri<ScheduleEntry[]>("get_schedule");

        if (!Array.isArray(data)) {
          const error = `Invalid schedule data format: ${typeof data}`;
          console.error(error, data);
          throw new Error(error);
        }

        console.log("Loaded schedule data successfully:", {
          scheduleCount: data.length,
          firstEntry: data[0],
        });

        setSchedule(data);
        setError(null);
      } catch (error) {
        const errorMessage =
          error instanceof Error ? error.message : "Unknown error";
        console.error("Failed to load schedule:", {
          error,
          message: errorMessage,
          stack: error instanceof Error ? error.stack : undefined,
        });
        setError(`Failed to load schedule: ${errorMessage}`);
        showToast(`Failed to load schedule: ${errorMessage}`, "error");
      } finally {
        setLoading(false);
      }
    };

    loadSchedule();
  }, [showToast]);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[calc(100vh-8rem)]">
        <Loader2 className="w-8 h-8 animate-spin text-blue-500" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center min-h-[calc(100vh-8rem)] space-y-4">
        <AlertCircle className="w-12 h-12 text-red-500" />
        <p className="text-lg font-medium text-gray-900">{error}</p>
        <button
          onClick={() => window.location.reload()}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-500 rounded-md hover:bg-blue-600"
        >
          Try Again
        </button>
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
      {schedule.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 space-y-4">
          <p className="text-lg font-medium text-gray-900">
            No scheduled episodes
          </p>
          <p className="text-sm text-gray-500">Check back later for updates</p>
        </div>
      ) : (
        <div className="space-y-4">
          {schedule.map((entry) => (
            <div
              key={`${entry.title}-${entry.episode}`}
              className="bg-white p-4 rounded-lg shadow-sm hover:shadow-md transition-shadow duration-200"
            >
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="text-lg font-semibold text-gray-900">
                    {entry.title}
                  </h3>
                  <p className="text-sm text-gray-600">
                    Episode {entry.episode}
                  </p>
                </div>
                <div className="text-sm text-gray-500">{entry.air_date}</div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
