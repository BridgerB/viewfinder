(ns viewfinder.sky
  "The skyline of a photograph: one pixel per column where the land meets the sky. Trees, a pole
   and a bright cloud are not the skyline. The trace is data the score consumes; it does not know
   which mountain it is looking at."
  (:require [clojure.java.io :as io])
  (:import [java.awt.image BufferedImage]))

(set! *warn-on-reflection* true)

(def rock-run 160) ; a hit that returns to sky within this many pixels is a cloud, not the ridge
(def columns 160) ; samples the score keeps; the rest of the columns are the same ridge

(defn read-image
  "An ARGB raster, {:sky/width :sky/height :sky/argb}, from a JPEG or PNG."
  [path]
  (let [^BufferedImage image (javax.imageio.ImageIO/read (io/file path))]
    (when (nil? image)
      (throw (ex-info "unreadable image" {:path path})))
    (let [width (.getWidth image)
          height (.getHeight image)
          argb (int-array (* width height))]
      (.getRGB image 0 0 width height argb 0 width)
      {:sky/width width :sky/height height :sky/argb argb})))

(defn- channel
  "One byte of an ARGB pixel. shift 16 is red, 8 green, 0 blue."
  [argb shift]
  (bit-and (unsigned-bit-shift-right argb shift) 0xff))

(defn- sky?
  [r g b]
  (and (> b (+ r 12)) (>= b (- g 8)) (> b 140)))

(defn- haze?
  "Bright and nearly colorless. Pale sky fails sky? when it is not blue enough, and a cloud edge
   is the same. A sunlit cliff is redder than this, so it stays part of the ridge."
  [r g b]
  (let [maxv (max r g b)
        minv (min r g b)
        sat (if (zero? maxv) 0.0 (/ (- maxv minv) maxv))
        lum (+ (* 0.299 r) (* 0.587 g) (* 0.114 b))]
    (and (> lum 155.0) (< sat 0.18))))

(defn- ridge-row
  "The row where column x meets land and does not return to sky, or nil."
  [^ints argb ^long width ^long height ^long x]
  (loop [y 0]
    (when (< y (- height rock-run))
      (let [p (aget argb (+ (* y width) x))
            r (channel p 16)
            g (channel p 8)
            b (channel p 0)]
        (cond
          (or (sky? r g b) (haze? r g b)) (recur (inc y))
          :else
          (let [back (loop [k 1]
                       (cond
                         (> k rock-run) false
                         :else
                         (let [q (aget argb (+ (* (+ y k) width) x))
                               qr (channel q 16)
                               qg (channel q 8)
                               qb (channel q 0)]
                           (cond
                             (sky? qr qg qb) true
                             :else (recur (inc k))))))]
            (cond
              back (recur (inc y))
              :else y)))))))

(defn- median-of
  [ys]
  (let [s (vec (sort ys))]
    (nth s (quot (count s) 2))))

(defn- fill-median
  "Each column replaced by the median of the rows within radius, ignoring gaps. A thin spike
   loses the vote. A gap takes the ridge beside it."
  [rows radius]
  (let [n (count rows)]
    (mapv (fn [x]
            (let [w (into []
                          (keep (fn [k]
                                  (let [xx (+ x k)]
                                    (when (<= 0 xx (dec n))
                                      (nth rows xx)))))
                          (range (- radius) (inc radius)))]
              (when (>= (count w) radius)
                (median-of w))))
          (range n))))

(defn- despike
  "Drops a column that sits jump pixels above the wide median. That is a pole, not a summit.
   y grows downward, so above means a smaller row."
  [rows radius jump]
  (let [wide (fill-median rows radius)]
    (mapv (fn [y w]
            (cond
              (nil? y) nil
              (nil? w) y
              (< y (- w jump)) nil
              :else y))
          rows
          wide)))

(defn trace
  "Skyline pixels {:sky/x :sky/y} in image coordinates. Columns whose ridge is in the top of the
   frame or down in the foreground are dropped."
  [{:sky/keys [width height argb]}]
  (let [^ints argb argb
        raw (mapv (fn [x] (ridge-row argb width height x)) (range width))
        cleaned (-> raw (despike 70 50) (fill-median 8))]
    (into []
          (keep-indexed
           (fn [x y]
             (when (and y
                        (< 0.15 (/ (double y) (double height)) 0.78))
               {:sky/x (+ (double x) 0.5) :sky/y (+ (double y) 0.5)})))
          cleaned)))

(defn decimate
  "About target points, evenly spaced along the trace."
  [skyline target]
  (let [n (count skyline)
        step (max 1 (long (Math/ceil (/ (double (max n 1)) (double target)))))]
    (into [] (take-nth step) skyline)))

(defn trace-file
  "trace of the image at path."
  [path]
  (trace (read-image path)))
