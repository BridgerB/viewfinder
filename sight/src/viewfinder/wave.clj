(ns viewfinder.wave
  "Independent places, scored together. A wave is 2000 places. The processor pool runs the wave,
   one thread per processor, and the next wave waits until this one is done. A query never holds
   a thread per place."
  (:import [java.util.concurrent Callable ExecutorService Executors Future]))

(set! *warn-on-reflection* true)

(def batch-size 2000) ; places submitted together; the pool, not this number, is the thread count

(defn across
  "f applied to each item. Items in a wave run on the processor pool and the wave finishes before
   the next one starts. Results come back in order. f is a pure function of one item."
  [f xs]
  (let [threads (.availableProcessors (Runtime/getRuntime))
        ^ExecutorService pool (Executors/newFixedThreadPool threads)]
    (try
      (into []
            (mapcat (fn [wave]
                      (let [futs (mapv (fn [item]
                                         (.submit pool
                                                  ^Callable
                                                  (reify Callable
                                                    (call [_] (f item)))))
                                       wave)]
                        (mapv (fn [^Future fut] (.get fut)) futs))))
            (partition-all batch-size xs))
      (finally
        (.shutdownNow pool)))))
