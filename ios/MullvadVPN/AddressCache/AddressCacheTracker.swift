//
//  AddressCacheTracker.swift
//  MullvadVPN
//
//  Created by pronebird on 08/12/2021.
//  Copyright © 2021 Mullvad VPN AB. All rights reserved.
//

import UIKit
import Logging

extension AddressCache {

    enum CacheUpdateResult {
        /// Address cache update was throttled as it was requested too early.
        case throttled

        /// Address cache is successfully updated.
        case finished
    }

    class Tracker {
        /// Shared instance.
        static let shared: AddressCache.Tracker = {
            return AddressCache.Tracker(
                apiProxy: REST.ProxyFactory.shared.createAPIProxy(),
                store: AddressCache.Store.shared
            )
        }()

        /// Update interval (in seconds).
        private static let updateInterval: TimeInterval = 60 * 60 * 24

        /// Retry interval (in seconds).
        private static let retryInterval: TimeInterval = 60 * 15

        /// Logger.
        private let logger = Logger(label: "AddressCache.Tracker")

        /// REST API proxy.
        private let apiProxy: REST.APIProxy

        /// Store.
        private let store: AddressCache.Store

        /// A flag that indicates whether periodic updates are running
        private var isPeriodicUpdatesEnabled = false

        /// The date of last failed attempt.
        private var lastFailureAttemptDate: Date?

        /// Timer used for scheduling periodic updates.
        private var timer: DispatchSourceTimer?

        /// Operation queue.
        private let operationQueue = OperationQueue()

        /// Lock used for synchronizing member access.
        private let nslock = NSLock()

        /// Designated initializer
        private init(apiProxy: REST.APIProxy, store: AddressCache.Store) {
            self.apiProxy = apiProxy
            self.store = store

            operationQueue.maxConcurrentOperationCount = 1
        }

        func startPeriodicUpdates() {
            nslock.lock()
            defer { nslock.unlock() }

            guard !isPeriodicUpdatesEnabled else {
                return
            }

            logger.debug("Start periodic address cache updates.")

            isPeriodicUpdatesEnabled = true

            let scheduleDate = _nextScheduleDate()

            logger.debug("Schedule address cache update on \(scheduleDate.logFormatDate()).")

            scheduleEndpointsUpdate(startTime: .now() + scheduleDate.timeIntervalSinceNow)
        }

        func stopPeriodicUpdates() {
            nslock.lock()
            defer { nslock.unlock() }

            guard isPeriodicUpdatesEnabled else { return }

            logger.debug("Stop periodic address cache updates.")

            isPeriodicUpdatesEnabled = false

            timer?.cancel()
            timer = nil
        }

        typealias UpdateEndpointsCompletionHandler = (
            _ completion: OperationCompletion<CacheUpdateResult, Error>
        ) -> Void

        func updateEndpoints(completionHandler: UpdateEndpointsCompletionHandler? = nil) -> Cancellable {
            let operation = ResultBlockOperation<CacheUpdateResult, Error>(dispatchQueue: nil) { operation in
                guard self.nextScheduleDate() <= Date() else {
                    operation.finish(completion: .success(.throttled))
                    return
                }

                let task = self.apiProxy.getAddressList(retryStrategy: .default) { completion in
                    operation.finish(
                        completion: self.handleResponse(completion: completion)
                    )
                }

                operation.addCancellationBlock {
                    task.cancel()
                }
            }

            operation.completionQueue = .main
            operation.completionHandler = completionHandler

            let backgroundTaskIdentifier = UIApplication.shared.beginBackgroundTask(withName: "AddressCache.Tracker.updateEndpoints") {
                operation.cancel()
            }

            operation.completionBlock = {
                UIApplication.shared.endBackgroundTask(backgroundTaskIdentifier)
            }

            operationQueue.addOperation(operation)

            return operation
        }

        func nextScheduleDate() -> Date {
            nslock.lock()
            defer { nslock.unlock() }

            return _nextScheduleDate()
        }

        private func handleResponse(
            completion: OperationCompletion<[AnyIPEndpoint], REST.Error>
        ) -> OperationCompletion<CacheUpdateResult, Error>
        {
            let mappedCompletion = completion
                .flatMapError { error -> OperationCompletion<[AnyIPEndpoint], REST.Error> in
                    if case URLError.cancelled = error {
                        return .cancelled
                    } else {
                        return .failure(error)
                    }
                }
                .tryMap { endpoints -> CacheUpdateResult in
                    try store.setEndpoints(endpoints)

                    return .finished
                }

            nslock.lock()
            lastFailureAttemptDate = mappedCompletion.isSuccess ? nil : Date()
            nslock.unlock()

            if let error = mappedCompletion.error {
                logger.error(
                    chainedError: AnyChainedError(error),
                    message: "Failed to update address cache."
                )
            }

            return mappedCompletion
        }

        private func scheduleEndpointsUpdate(startTime: DispatchWallTime) {
            let newTimer = DispatchSource.makeTimerSource()
            newTimer.setEventHandler { [weak self] in
                self?.handleTimer()
            }

            newTimer.schedule(wallDeadline: startTime)
            newTimer.activate()

            timer?.cancel()
            timer = newTimer
        }

        private func handleTimer() {
            _ = updateEndpoints { result in
                self.nslock.lock()
                defer { self.nslock.unlock() }

                guard self.isPeriodicUpdatesEnabled else { return }

                let scheduleDate = self._nextScheduleDate()

                self.logger.debug("Schedule next address cache update on \(scheduleDate.logFormatDate())")

                self.scheduleEndpointsUpdate(startTime: .now() + scheduleDate.timeIntervalSinceNow)
            }
        }

        private func _nextScheduleDate() -> Date {
            let nextDate = lastFailureAttemptDate.map { date in
                return Date(
                    timeInterval: Self.retryInterval,
                    since: date
                )
            } ?? Date(
                timeInterval: Self.updateInterval,
                since: store.getLastUpdateDate()
            )

            return max(nextDate, Date())
        }

    }
}
