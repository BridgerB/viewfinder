(ns viewfinder.field
  "The horizon field: one packed record per place, built once and read on every query. The file
   is binary. Lat, lon and ground elevation sit beside 360 elevation angles in centidegrees
   and the meters to the sample that won each degree. A missing ray is the short sentinel,
   not a zero, and its distance is zero."
  (:require [clojure.java.io :as io]
            [clojure.math :as math]
            [viewfinder.dem :as dem]
            [viewfinder.horizon :as horizon]
            [viewfinder.wave :as wave])
  (:import [java.nio ByteBuffer ByteOrder]
           [java.nio.channels FileChannel]
           [java.nio.file StandardOpenOption]))

(set! *warn-on-reflection* true)

(def magic (byte-array (map byte "HZN2")))
(def header-bytes 12) ; magic, count, azimuths
(def pose-bytes 20) ; double lat, double lon, float ground
(def record-bytes (+ pose-bytes (* horizon/azimuths 2) (* horizon/azimuths 4)))

(defn lattice
  "Places on a step-m grid inside the box, as [lat lon]. The step is in meters of latitude;
   longitude is stretched by cos of the box center so the cells stay nearly square."
  [south north west east step-m]
  (let [lat-m 111320.0
        mid (/ (+ south north) 2.0)
        lon-m (* lat-m (math/cos (math/to-radians mid)))
        dlat (/ step-m lat-m)
        dlon (/ step-m lon-m)]
    (for [lat (range (+ south (/ dlat 2.0)) north dlat)
          lon (range (+ west (/ dlon 2.0)) east dlon)]
      [(double lat) (double lon)])))

(defn- record
  "One place's bytes: lat, lon, ground meters, an angle per degree, then meters to that sample.
   Nil when the place has no ground."
  [grid lat lon]
  (when-let [ground (dem/elev-at grid lat lon)]
    (when-let [seen (horizon/view grid lat lon)]
      (let [^shorts angles (:horizon/angles seen)
            ^floats distances (:horizon/distances seen)
            buf (-> (ByteBuffer/allocate record-bytes) (.order ByteOrder/LITTLE_ENDIAN))]
        (.putDouble buf (double lat))
        (.putDouble buf (double lon))
        (.putFloat buf (float ground))
        (.put (.asShortBuffer buf) angles)
        (.position buf (int (+ pose-bytes (* horizon/azimuths 2))))
        (.put (.asFloatBuffer buf) distances)
        (.array buf)))))

(defn build
  "Write the field for every place the grid can see. Places are independent and run in waves.
   Prints progress. Returns the number of records written."
  [grid places out]
  (let [places (vec places)
        total (count places)
        done (atom 0)
        records (into []
                      (remove nil?)
                      (wave/across
                       (fn [[lat lon]]
                         (let [rec (record grid lat lon)
                               k (swap! done inc)]
                           (when (zero? (mod k 25))
                             (println {:horizon k :of total})
                             (flush))
                           rec))
                       places))
        n (count records)
        header (-> (ByteBuffer/allocate header-bytes) (.order ByteOrder/LITTLE_ENDIAN))]
    (.put header ^"[B" magic)
    (.putInt header n)
    (.putInt header horizon/azimuths)
    (with-open [ch (FileChannel/open (.toPath (io/file out))
                                     (into-array StandardOpenOption
                                                 [StandardOpenOption/CREATE
                                                  StandardOpenOption/WRITE
                                                  StandardOpenOption/TRUNCATE_EXISTING]))]
      (.write ch (.flip header))
      (doseq [rec records]
        (.write ch (ByteBuffer/wrap rec))))
    n))

(defn read-field
  "The packed field at path: arrays of lat, lon, ground, a short array of angles, and a float
   array of distances in meters. Both horizon arrays are place-major, horizon/azimuths per place."
  [path]
  (let [ch (FileChannel/open (.toPath (io/file path))
                             (into-array StandardOpenOption [StandardOpenOption/READ]))
        size (.size ch)
        buf (-> (ByteBuffer/allocate (int size)) (.order ByteOrder/LITTLE_ENDIAN))]
    (try (while (.hasRemaining buf) (.read ch buf))
         (finally (.close ch)))
    (.flip buf)
    (let [tag (byte-array 4)]
      (.get buf tag)
      (when-not (java.util.Arrays/equals ^"[B" tag ^"[B" magic)
        (throw (ex-info "not a horizon field" {:path path})))
      (let [n (.getInt buf)
            azimuths (.getInt buf)
            lats (double-array n)
            lons (double-array n)
            grounds (float-array n)
            angles (short-array (* n azimuths))
            distances (float-array (* n azimuths))]
        (dotimes [i n]
          (aset lats i (.getDouble buf))
          (aset lons i (.getDouble buf))
          (aset grounds i (.getFloat buf))
          (let [shorts (.asShortBuffer buf)]
            (.get shorts angles (* i azimuths) azimuths))
          (.position buf (int (+ (.position buf) (* azimuths 2))))
          (let [spans (.asFloatBuffer buf)]
            (.get spans distances (* i azimuths) azimuths))
          (.position buf (int (+ (.position buf) (* azimuths 4)))))
        {:field/count n
         :field/azimuths azimuths
         :field/lats lats
         :field/lons lons
         :field/grounds grounds
         :field/angles angles
         :field/distances distances}))))

(defn gather
  "The field for places, walked now and held in memory: the map read-field returns. Places are
   independent and run in waves. A place with no ground is left out. This is how a search looks
   closer than the stored lattice."
  [grid places]
  (let [seen (into []
                   (remove nil?)
                   (wave/across
                    (fn [[lat lon]]
                      (when-let [ground (dem/elev-at grid lat lon)]
                        (when-let [view (horizon/view grid lat lon)]
                          {:lat lat :lon lon :ground ground :view view})))
                    places))
        n (count seen)
        azimuths (int horizon/azimuths)
        ^doubles lats (double-array n)
        ^doubles lons (double-array n)
        ^floats grounds (float-array n)
        ^shorts angles (short-array (* n azimuths))
        ^floats distances (float-array (* n azimuths))]
    (dotimes [i n]
      (let [{:keys [lat lon ground view]} (nth seen i)]
        (aset lats i (double lat))
        (aset lons i (double lon))
        (aset grounds i (float ground))
        (System/arraycopy (:horizon/angles view) 0 angles (* i azimuths) azimuths)
        (System/arraycopy (:horizon/distances view) 0 distances (* i azimuths) azimuths)))
    {:field/count n
     :field/azimuths azimuths
     :field/lats lats
     :field/lons lons
     :field/grounds grounds
     :field/angles angles
     :field/distances distances}))
