(ns viewfinder.dem
  "A bare-earth grid: a header and a short array of elevations in meters. Off the grid and nodata
   are nil, never zero, so a horizon ray can tell that it has left the loaded land."
  (:require [clojure.java.io :as io]
            [clojure.string :as str])
  (:import [java.nio ByteBuffer ByteOrder]
           [java.nio.channels FileChannel]
           [java.nio.file StandardOpenOption]))

(set! *warn-on-reflection* true)

(def nodata -32768) ; stored where the source has no ground; never a real Utah elevation

(defn grid
  "A grid whose row 0 is the north edge. dy is degrees per row and is negative when rows run
   south. samples is a short array, row-major, one elevation in meters per cell."
  [north west dx dy width samples]
  (let [n (alength ^shorts samples)]
    {:dem/north (double north)
     :dem/west (double west)
     :dem/dx (double dx)
     :dem/dy (double dy)
     :dem/width (int width)
     :dem/height (int (quot n width))
     :dem/samples samples}))

(defn- sample-at
  "Meters at row col, or nodata when the cell is missing or off the array."
  [^shorts samples width height row col]
  (if (or (neg? row) (neg? col) (>= row height) (>= col width))
    nodata
    (let [v (aget samples (+ (* row width) col))]
      (if (= v nodata) nodata v))))

(defn elev-at
  "Meters at lat lon, bilinear between the four surrounding cells, or nil if any of them is
   missing."
  [g lat lon]
  (let [dx (double (:dem/dx g))
        dy (double (:dem/dy g))
        width (int (:dem/width g))
        height (int (:dem/height g))
        ^shorts samples (:dem/samples g)
        col (/ (- (double lon) (double (:dem/west g))) dx)
        row (/ (- (double lat) (double (:dem/north g))) dy)]
    (when (and (>= col 0.0) (>= row 0.0)
               (< col (dec width)) (< row (dec height)))
      (let [c0 (int (Math/floor col))
            r0 (int (Math/floor row))
            tc (- col c0)
            tr (- row r0)
            v00 (sample-at samples width height r0 c0)
            v10 (sample-at samples width height r0 (inc c0))
            v01 (sample-at samples width height (inc r0) c0)
            v11 (sample-at samples width height (inc r0) (inc c0))]
        (when-not (or (= v00 nodata) (= v10 nodata) (= v01 nodata) (= v11 nodata))
          (+ (* (- 1.0 tr) (+ (* (- 1.0 tc) v00) (* tc v10)))
             (* tr (+ (* (- 1.0 tc) v01) (* tc v11)))))))))

(defn- hdr-entries
  "ENVI text as {key value}, keys lower case, values trimmed."
  [text]
  (into {}
        (keep (fn [line]
                (let [[k v] (str/split line #"=" 2)]
                  (when v [(str/lower-case (str/trim k)) (str/trim v)]))))
        (str/split-lines text)))

(defn- map-info
  "Tie point and pixel size from an ENVI `map info` value. The numbers are reference pixel,
   reference line, tie longitude, tie latitude, then pixel size. A datum year later in the line
   is not a pixel size. dy comes back negative: row 0 is the north edge, so latitude falls as
   the row rises."
  [text]
  (let [nums (mapv parse-double (re-seq #"-?[0-9]+(?:\.[0-9]+)?(?:[eE][-+]?[0-9]+)?" text))]
    (when (< (count nums) 6)
      (throw (ex-info "ENVI map info has no tie point" {:map-info text})))
    {:west (nth nums 2)
     :north (nth nums 3)
     :dx (nth nums 4)
     :dy (- (Math/abs (double (nth nums 5))))}))

(defn south
  "Latitude of the south edge. Row 0 is the north edge and dy is negative."
  [g]
  (+ (double (:dem/north g)) (* (double (:dem/dy g)) (double (:dem/height g)))))

(defn east
  "Longitude of the east edge."
  [g]
  (+ (double (:dem/west g)) (* (double (:dem/dx g)) (double (:dem/width g)))))

(defn read-envi
  "The grid stored as an ENVI pair, path.hdr beside path. Int16, little or big endian as the
   header says, band sequential. Nodata cells are already nodata in the file, or were translated
   to it."
  [path]
  (let [hdr (hdr-entries (slurp (str path ".hdr")))
        _ (when-not (= "2" (get hdr "data type"))
            (throw (ex-info "ENVI grid is not int16" {:data-type (get hdr "data type")})))
        _ (when-not (= "bsq" (str/lower-case (get hdr "interleave" "")))
            (throw (ex-info "ENVI grid is not band sequential" {:interleave (get hdr "interleave")})))
        width (parse-long (get hdr "samples"))
        height (parse-long (get hdr "lines"))
        info (map-info (get hdr "map info"))
        order (if (= "1" (get hdr "byte order")) ByteOrder/BIG_ENDIAN ByteOrder/LITTLE_ENDIAN)
        n (* width height)
        buf (-> (ByteBuffer/allocate (int (* n 2))) (.order order))
        ch (FileChannel/open (.toPath (io/file path))
                             (into-array [StandardOpenOption/READ]))]
    (try (while (.hasRemaining buf) (.read ch buf))
         (finally (.close ch)))
    (.flip buf)
    (let [samples (short-array n)]
      (.get (.asShortBuffer buf) samples)
      (grid (:north info) (:west info) (:dx info) (:dy info) width samples))))
